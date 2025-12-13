use crate::models::Package;
use anyhow::Result;
use async_trait::async_trait;

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

    /// Downgrade/revert a package to a specific version/revision (optional per backend)
    async fn downgrade_to(&self, _name: &str, _version: &str) -> Result<()> {
        anyhow::bail!("Downgrade to a specific version is not supported for this source")
    }

    /// List available downgrade target versions (optional per backend)
    async fn available_downgrade_versions(&self, _name: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// Search for new packages
    async fn search(&self, query: &str) -> Result<Vec<Package>>;
}

// We need async_trait for async trait methods
#[macro_export]
macro_rules! impl_async_trait {
    () => {
        use async_trait::async_trait;
    };
}
