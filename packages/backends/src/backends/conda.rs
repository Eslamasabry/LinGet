use crate::backends::PackageBackend;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus};

pub struct CondaBackend;

impl CondaBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CondaBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for CondaBackend {
    fn is_available() -> bool {
        which::which("conda").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = std::process::Command::new("conda")
            .args(["list", "--json"])
            .output()?;

        Ok(Vec::new())
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }

    async fn install(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("conda")
            .args(["install", "-y", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("conda install failed");
        }

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("conda")
            .args(["remove", "-y", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("conda remove failed");
        }

        Ok(())
    }

    async fn update(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("conda")
            .args(["update", "-y", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("conda update failed");
        }

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = std::process::Command::new("conda")
            .args(["search", query, "--json"])
            .output()?;

        Ok(Vec::new())
    }

    fn source(&self) -> PackageSource {
        PackageSource::Conda
    }
}
