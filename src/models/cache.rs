use crate::models::{Config, Package};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Cached package data for instant loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageCache {
    /// When the cache was last updated
    pub last_updated: DateTime<Utc>,
    /// Cached packages
    pub packages: Vec<Package>,
}

impl PackageCache {
    /// Maximum age of cache before it's considered stale (in seconds)
    const MAX_AGE_SECS: i64 = 3600; // 1 hour

    pub fn cache_path() -> PathBuf {
        Config::config_dir().join("package_cache.json")
    }

    /// Load cache from disk
    pub fn load() -> Option<Self> {
        let path = Self::cache_path();
        if !path.exists() {
            tracing::debug!("No cache file found");
            return None;
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(cache) => {
                    tracing::debug!("Loaded package cache");
                    Some(cache)
                }
                Err(e) => {
                    tracing::warn!("Failed to parse cache: {}", e);
                    None
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read cache: {}", e);
                None
            }
        }
    }

    /// Save cache to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let dir = Config::config_dir();
        std::fs::create_dir_all(&dir)?;

        let content = serde_json::to_string(self)?;
        std::fs::write(Self::cache_path(), content)?;
        tracing::debug!("Saved package cache with {} packages", self.packages.len());
        Ok(())
    }

    /// Check if cache is stale
    pub fn is_stale(&self) -> bool {
        let age = Utc::now().signed_duration_since(self.last_updated);
        age.num_seconds() > Self::MAX_AGE_SECS
    }

    /// Create a new cache from packages
    pub fn new(packages: Vec<Package>) -> Self {
        Self {
            last_updated: Utc::now(),
            packages,
        }
    }

    /// Save packages to cache
    pub fn save_packages(packages: &[Package]) {
        let cache = Self::new(packages.to_vec());
        if let Err(e) = cache.save() {
            tracing::warn!("Failed to save cache: {}", e);
        }
    }
}
