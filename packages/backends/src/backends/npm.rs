use crate::backends::PackageBackend;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus};
use std::path::PathBuf;

pub struct NpmBackend;

impl NpmBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NpmBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for NpmBackend {
    fn is_available() -> bool {
        which::which("npm").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = std::process::Command::new("npm")
            .args(["list", "-g", "--depth=0", "--json"])
            .output()?;

        Ok(Vec::new())
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }

    async fn install(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("npm")
            .args(["install", "-g", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("npm install failed");
        }

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("npm")
            .args(["uninstall", "-g", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("npm uninstall failed");
        }

        Ok(())
    }

    async fn update(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("npm")
            .args(["update", "-g", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("npm update failed");
        }

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = std::process::Command::new("npm")
            .args(["search", query, "--json"])
            .output()?;

        Ok(Vec::new())
    }

    fn source(&self) -> PackageSource {
        PackageSource::Npm
    }

    async fn get_package_commands(&self, _name: &str) -> Result<Vec<(String, PathBuf)>> {
        Ok(Vec::new())
    }
}
