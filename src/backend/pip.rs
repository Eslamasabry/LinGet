use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

pub struct PipBackend;

impl PipBackend {
    pub fn new() -> Self {
        Self
    }

    fn get_pip_command() -> &'static str {
        // Try pip3 first, then pip
        if which::which("pip3").is_ok() {
            "pip3"
        } else {
            "pip"
        }
    }
}

impl Default for PipBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct PipPackageInfo {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct PipOutdatedInfo {
    name: String,
    version: String,
    latest_version: String,
}

#[async_trait]
impl PackageBackend for PipBackend {
    fn is_available() -> bool {
        which::which("pip3").is_ok() || which::which("pip").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let pip = Self::get_pip_command();
        let output = Command::new(pip)
            .args(["list", "--format=json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list pip packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        if let Ok(parsed) = serde_json::from_str::<Vec<PipPackageInfo>>(&stdout) {
            for pkg in parsed {
                packages.push(Package {
                    name: pkg.name,
                    version: pkg.version,
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
                });
            }
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        let pip = Self::get_pip_command();
        let output = Command::new(pip)
            .args(["list", "--outdated", "--format=json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check pip updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        if let Ok(parsed) = serde_json::from_str::<Vec<PipOutdatedInfo>>(&stdout) {
            for pkg in parsed {
                packages.push(Package {
                    name: pkg.name,
                    version: pkg.version,
                    available_version: Some(pkg.latest_version),
                    description: String::new(),
                    source: PackageSource::Pip,
                    status: PackageStatus::UpdateAvailable,
                    size: None,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                });
            }
        }

        Ok(packages)
    }

    async fn install(&self, name: &str) -> Result<()> {
        let pip = Self::get_pip_command();
        let status = Command::new(pip)
            .args(["install", "--user", name])
            .status()
            .await
            .context("Failed to install pip package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install pip package {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let pip = Self::get_pip_command();
        let status = Command::new(pip)
            .args(["uninstall", "-y", name])
            .status()
            .await
            .context("Failed to remove pip package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove pip package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let pip = Self::get_pip_command();
        let status = Command::new(pip)
            .args(["install", "--user", "--upgrade", name])
            .status()
            .await
            .context("Failed to update pip package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update pip package {}", name)
        }
    }
}
