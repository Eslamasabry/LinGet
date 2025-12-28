use crate::models::{Package, PackageSource, Repository};
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct LockStatus {
    pub is_locked: bool,
    pub lock_holder: Option<String>,
    pub lock_files: Vec<PathBuf>,
}

/// Trait that all package manager backends must implement
#[async_trait]
pub trait PackageBackend: Send + Sync {
    /// Check if this backend is available on the system
    fn is_available() -> bool
    where
        Self: Sized;

    /// List all installed packages
    async fn list_installed(&self) -> Result<Vec<Package>>;

    /// Check for available updates
    async fn check_updates(&self) -> Result<Vec<Package>>;

    /// Install a package by name
    async fn install(&self, name: &str) -> Result<()>;

    /// Remove a package by name
    async fn remove(&self, name: &str) -> Result<()>;

    /// Update a package by name
    async fn update(&self, name: &str) -> Result<()>;

    /// Downgrade/revert a package by name (best-effort; optional per backend)
    async fn downgrade(&self, _name: &str) -> Result<()> {
        anyhow::bail!("Downgrade is not supported for this source")
    }

    #[allow(dead_code)]
    async fn downgrade_to(&self, _name: &str, _version: &str) -> Result<()> {
        anyhow::bail!("Downgrade to a specific version is not supported for this source")
    }

    #[allow(dead_code)]
    async fn available_downgrade_versions(&self, _name: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// Get changelog/release notes for a package (optional per backend)
    /// Returns markdown-formatted changelog if available
    async fn get_changelog(&self, _name: &str) -> Result<Option<String>> {
        Ok(None)
    }

    /// List configured repositories (optional per backend)
    async fn list_repositories(&self) -> Result<Vec<Repository>> {
        Ok(Vec::new())
    }

    #[allow(dead_code)]
    async fn add_repository(&self, _url: &str, _name: Option<&str>) -> Result<()> {
        anyhow::bail!("Adding repositories is not supported for this source")
    }

    #[allow(dead_code)]
    async fn remove_repository(&self, _name: &str) -> Result<()> {
        anyhow::bail!("Removing repositories is not supported for this source")
    }

    /// Search for new packages
    async fn search(&self, query: &str) -> Result<Vec<Package>>;

    /// Get the size of the package cache in bytes (for cleanup tools)
    async fn get_cache_size(&self) -> Result<u64> {
        Ok(0)
    }

    /// Get packages that are no longer needed (orphaned dependencies)
    async fn get_orphaned_packages(&self) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }

    /// Clean up package cache, returns bytes freed
    async fn cleanup_cache(&self) -> Result<u64> {
        Ok(0)
    }

    /// Get packages that depend on the given package (reverse dependencies)
    async fn get_reverse_dependencies(&self, _name: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    #[allow(dead_code)]
    fn source(&self) -> PackageSource;

    async fn get_package_commands(&self, _name: &str) -> Result<Vec<(String, PathBuf)>> {
        Ok(Vec::new())
    }

    async fn check_lock_status(&self) -> LockStatus {
        LockStatus::default()
    }
}

// We need async_trait for async trait methods
#[macro_export]
macro_rules! impl_async_trait {
    () => {
        use async_trait::async_trait;
    };
}
