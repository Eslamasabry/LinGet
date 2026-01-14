use crate::backends::PackageBackend;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus};

pub struct PipBackend;

impl PipBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PipBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for PipBackend {
    fn is_available() -> bool {
        which::which("pip3").is_ok() || which::which("pip").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = std::process::Command::new("pip3")
            .args(["list", "--format=freeze"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            if let Some((name, version)) = line.split_once('=') {
                packages.push(Package {
                    name: name.to_string(),
                    version: version.trim_start_matches('-').to_string(),
                    available_version: None,
                    description: String::new(),
                    source: PackageSource::Pip,
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
        let output = std::process::Command::new("pip3")
            .args(["list", "--outdated", "--format=freeze"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            if let Some((name, version)) = line.split_once('=') {
                let latest = std::process::Command::new("pip3")
                    .args(["show", name])
                    .output()?;

                let latest_str = String::from_utf8_lossy(&latest.stdout);
                let latest_version = latest_str
                    .lines()
                    .find(|l| l.starts_with("Version:"))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| version.trim_start_matches('-').to_string());

                packages.push(Package {
                    name: name.to_string(),
                    version: version.trim_start_matches('-').to_string(),
                    available_version: Some(latest_version),
                    description: String::new(),
                    source: PackageSource::Pip,
                    status: PackageStatus::UpdateAvailable,
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

    async fn install(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pip3")
            .args(["install", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("pip install failed");
        }

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pip3")
            .args(["uninstall", "-y", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("pip uninstall failed");
        }

        Ok(())
    }

    async fn update(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pip3")
            .args(["install", "--upgrade", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("pip upgrade failed");
        }

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = std::process::Command::new("pip3")
            .args(["search", query])
            .output()?;

        Ok(Vec::new())
    }

    fn source(&self) -> PackageSource {
        PackageSource::Pip
    }
}
