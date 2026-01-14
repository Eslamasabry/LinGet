use crate::streaming::StreamLine;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, Repository};
use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Default)]
pub struct LockStatus {
    pub is_locked: bool,
    pub lock_holder: Option<String>,
    pub lock_files: Vec<PathBuf>,
}

#[async_trait]
pub trait PackageBackend: Send + Sync {
    fn is_available() -> bool
    where
        Self: Sized;

    async fn list_installed(&self) -> Result<Vec<Package>>;

    async fn check_updates(&self) -> Result<Vec<Package>>;

    async fn install(&self, name: &str) -> Result<()>;

    async fn install_streaming(
        &self,
        name: &str,
        _log_sender: Option<mpsc::Sender<StreamLine>>,
    ) -> Result<()> {
        self.install(name).await
    }

    async fn remove(&self, name: &str) -> Result<()>;

    async fn remove_streaming(
        &self,
        name: &str,
        _log_sender: Option<mpsc::Sender<StreamLine>>,
    ) -> Result<()> {
        self.remove(name).await
    }

    async fn update(&self, name: &str) -> Result<()>;

    async fn update_streaming(
        &self,
        name: &str,
        _log_sender: Option<mpsc::Sender<StreamLine>>,
    ) -> Result<()> {
        self.update(name).await
    }

    async fn downgrade(&self, _name: &str) -> Result<()> {
        anyhow::bail!("Downgrade is not supported for this source")
    }

    async fn downgrade_to(&self, _name: &str, _version: &str) -> Result<()> {
        anyhow::bail!("Downgrade to a specific version is not supported for this source")
    }

    async fn available_downgrade_versions(&self, _name: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    async fn get_changelog(&self, _name: &str) -> Result<Option<String>> {
        Ok(None)
    }

    async fn list_repositories(&self) -> Result<Vec<Repository>> {
        Ok(Vec::new())
    }

    async fn add_repository(&self, _url: &str, _name: Option<&str>) -> Result<()> {
        anyhow::bail!("Adding repositories is not supported for this source")
    }

    async fn remove_repository(&self, _name: &str) -> Result<()> {
        anyhow::bail!("Removing repositories is not supported for this source")
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>>;

    async fn get_cache_size(&self) -> Result<u64> {
        Ok(0)
    }

    async fn get_orphaned_packages(&self) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }

    async fn cleanup_cache(&self) -> Result<u64> {
        Ok(0)
    }

    async fn get_reverse_dependencies(&self, _name: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    fn source(&self) -> PackageSource;

    async fn get_package_commands(&self, _name: &str) -> Result<Vec<(String, PathBuf)>> {
        Ok(Vec::new())
    }

    async fn check_lock_status(&self) -> LockStatus {
        LockStatus::default()
    }
}
