use crate::backends::PackageBackend;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus};

pub struct BrewBackend;

impl BrewBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BrewBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for BrewBackend {
    fn is_available() -> bool {
        which::which("brew").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = std::process::Command::new("brew")
            .args(["list", "--versions"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: parts.get(1).unwrap_or(&"").to_string(),
                    available_version: None,
                    description: String::new(),
                    source: PackageSource::Brew,
                    status: PackageStatus::Installed,
                    size: None,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                    update_category: None,
                });
            }
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }

    async fn install(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("brew")
            .args(["install", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("brew install failed");
        }

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("brew")
            .args(["remove", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("brew remove failed");
        }

        Ok(())
    }

    async fn update(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("brew")
            .args(["upgrade", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("brew upgrade failed");
        }

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = std::process::Command::new("brew")
            .args(["search", query])
            .output()?;

        Ok(Vec::new())
    }

    fn source(&self) -> PackageSource {
        PackageSource::Brew
    }
}
