//! Short-TTL cache of upstream "latest version" lookups (PyPI, npm registry).
//!
//! A fresh update check spends most of its wall time asking indexes what the
//! latest version of unchanged packages is. Answers change rarely, so they
//! are cached with a TTL; on expiry only that package is re-queried. The
//! cache maps `<registry>:<name> -> (version, fetched_at)` and lives in the
//! data dir next to the catalog cache.

use anyhow::Context;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

const TTL_SECS: u64 = 60 * 60;
const CACHE_FILE: &str = "latest-versions.json";
const CACHE_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct LatestCache {
    version: u32,
    entries: HashMap<String, CachedEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedEntry {
    version: String,
    fetched_at: u64,
}

impl LatestCache {
    fn empty() -> Self {
        Self {
            version: CACHE_VERSION,
            entries: HashMap::new(),
        }
    }
}

fn cache_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("LINGET_DATA_DIR") {
        return PathBuf::from(dir).join(CACHE_FILE);
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("linget")
        .join(CACHE_FILE)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn cache() -> &'static Mutex<LatestCache> {
    static CACHE: OnceLock<Mutex<LatestCache>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let loaded = std::fs::read_to_string(cache_path())
            .ok()
            .and_then(|content| serde_json::from_str::<LatestCache>(&content).ok())
            .filter(|cache| cache.version == CACHE_VERSION);
        Mutex::new(loaded.unwrap_or_else(LatestCache::empty))
    })
}

/// Returns the cached latest version for `key` if it is still within TTL.
pub fn get(key: &str) -> Option<String> {
    let cache = cache().lock();
    let entry = cache.entries.get(key)?;
    is_fresh(entry.fetched_at).then(|| entry.version.clone())
}

fn is_fresh(fetched_at: u64) -> bool {
    now_secs().saturating_sub(fetched_at) < TTL_SECS
}

/// Records a fresh latest-version lookup and schedules a disk persist.
pub fn put(key: String, version: String) {
    {
        let mut cache = cache().lock();
        cache.entries.insert(
            key,
            CachedEntry {
                version,
                fetched_at: now_secs(),
            },
        );
    }
    persist_async();
}

/// Reads a whole-check result cached by `put_json` (e.g. a backend's full
/// update list). Bounded by the same TTL as version lookups.
pub fn get_json<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    let raw = get(key)?;
    serde_json::from_str(&raw).ok()
}

/// Caches a whole-check result, TTL-bounded like version lookups.
pub fn put_json<T: serde::Serialize>(key: &str, value: &T) {
    if let Ok(raw) = serde_json::to_string(value) {
        put(key.to_string(), raw);
    }
}

fn persist_async() {
    let snapshot = {
        let cache = cache().lock();
        serde_json::to_string(&*cache).context("failed to serialize latest-version cache")
    };
    if let Ok(content) = snapshot {
        tokio::task::spawn_blocking(move || {
            let path = cache_path();
            if let Some(parent) = path.parent() {
                if let Err(error) = std::fs::create_dir_all(parent) {
                    tracing::debug!(error = %error, "failed to create latest-cache directory");
                    return;
                }
            }
            if let Err(error) = std::fs::write(&path, content) {
                tracing::debug!(error = %error, "failed to write latest-version cache");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshness_boundary_is_the_ttl() {
        assert!(is_fresh(now_secs()));
        assert!(is_fresh(now_secs() - TTL_SECS + 1));
        assert!(!is_fresh(now_secs() - TTL_SECS));
        assert!(!is_fresh(0));
    }

    #[test]
    fn put_then_get_round_trips_within_a_process() {
        let dir = std::env::temp_dir().join(format!("linget-latest-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("LINGET_DATA_DIR", &dir);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            put("pypi:demo".to_string(), "2.0".to_string());
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        });

        assert_eq!(get("pypi:demo").as_deref(), Some("2.0"));
        assert_eq!(get("pypi:missing"), None);

        std::env::remove_var("LINGET_DATA_DIR");
        std::fs::remove_dir_all(&dir).ok();
    }
}
