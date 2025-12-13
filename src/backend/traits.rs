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
}

// We need async_trait for async trait methods
#[macro_export]
macro_rules! impl_async_trait {
    () => {
        use async_trait::async_trait;
    };
}
