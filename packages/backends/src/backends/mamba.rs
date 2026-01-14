use crate::backends::PackageBackend;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus};

pub struct MambaBackend;

impl MambaBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MambaBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for MambaBackend {
    fn is_available() -> bool {
        which::which("mamba").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = std::process::Command::new("mamba")
            .args(["list", "--json"])
            .output()?;

        Ok(Vec::new())
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }

    async fn install(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("mamba")
            .args(["install", "-y", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("mamba install failed");
        }

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("mamba")
            .args(["remove", "-y", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("mamba remove failed");
        }

        Ok(())
    }

    async fn update(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("mamba")
            .args(["update", "-y", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("mamba update failed");
        }

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = std::process::Command::new("mamba")
            .args(["search", query, "--json"])
            .output()?;

        Ok(Vec::new())
    }

    fn source(&self) -> PackageSource {
        PackageSource::Mamba
    }
}
