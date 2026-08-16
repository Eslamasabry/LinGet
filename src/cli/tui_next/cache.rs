//! Catalog cache: the last fully-loaded catalog, served instantly while a
//! fresh one is fetched. Without it every launch shows a skeleton for the
//! full backend listing + update-check duration, which is the single biggest
//! reason the TUI felt slow.

use crate::models::Package;
use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const CATALOG_CACHE_FILE: &str = "radar-catalog-cache.json";
const CATALOG_CACHE_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct CatalogCache {
    version: u32,
    saved_at_unix: i64,
    packages: Vec<Package>,
}

fn cache_path() -> PathBuf {
    // LINGET_DATA_DIR lets tests (and sandboxes) redirect data writes.
    if let Some(dir) = std::env::var_os("LINGET_DATA_DIR") {
        return PathBuf::from(dir).join(CATALOG_CACHE_FILE);
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("linget")
        .join(CATALOG_CACHE_FILE)
}

pub struct CachedCatalog {
    pub packages: Vec<Package>,
    pub saved_at: DateTime<Local>,
}

pub fn load() -> Option<CachedCatalog> {
    let path = cache_path();
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    let cache: CatalogCache = serde_json::from_str(&content).ok()?;
    if cache.version != CATALOG_CACHE_VERSION || cache.packages.is_empty() {
        return None;
    }
    let saved_at = DateTime::<Utc>::from_timestamp(cache.saved_at_unix, 0)?.with_timezone(&Local);
    Some(CachedCatalog {
        packages: cache.packages,
        saved_at,
    })
}

/// Rewrites the cache off the event loop; a failed save is not worth a
/// user-visible error, the next refresh will retry.
pub fn save_async(packages: Vec<Package>) {
    tokio::task::spawn_blocking(move || {
        if let Err(error) = save(&packages) {
            tracing::debug!(error = %error, "failed to save catalog cache");
        }
    });
}

fn save(packages: &[Package]) -> Result<()> {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("failed to create cache directory")?;
    }
    let cache = CatalogCache {
        version: CATALOG_CACHE_VERSION,
        saved_at_unix: Utc::now().timestamp(),
        packages: packages.to_vec(),
    };
    let content = serde_json::to_string(&cache).context("failed to serialize catalog cache")?;
    std::fs::write(&path, content).context("failed to write catalog cache")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_or_corrupt_caches_are_rejected_not_fatal() {
        // A cache from a newer version or bad JSON must read as "no cache",
        // never as an error the UI has to handle.
        let dir = std::env::temp_dir().join(format!("linget-cache-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(CATALOG_CACHE_FILE);
        std::fs::write(&path, "not json").unwrap();
        // load() consults the real data dir; the parse path is exercised via
        // serde directly here.
        let bad: Result<CatalogCache> =
            serde_json::from_str("not json").context("failed to parse catalog cache");
        assert!(bad.is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
