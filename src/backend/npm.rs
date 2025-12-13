use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

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

#[derive(Debug, Deserialize)]
struct NpmListOutput {
    dependencies: Option<std::collections::HashMap<String, NpmPackageInfo>>,
}

#[derive(Debug, Deserialize)]
struct NpmPackageInfo {
    version: Option<String>,
    _resolved: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NpmOutdatedEntry {
    current: Option<String>,
    _wanted: Option<String>,
    latest: Option<String>,
}

#[async_trait]
impl PackageBackend for NpmBackend {
    fn is_available() -> bool {
        which::which("npm").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("npm")
            .args(["list", "-g", "--depth=0", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list npm packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        if let Ok(parsed) = serde_json::from_str::<NpmListOutput>(&stdout) {
            if let Some(deps) = parsed.dependencies {
                for (name, info) in deps {
                    packages.push(Package {
                        name,
                        version: info.version.unwrap_or_default(),
                        available_version: None,
                        description: String::new(),
                        source: PackageSource::Npm,
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
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        let output = Command::new("npm")
            .args(["outdated", "-g", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check npm updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        if let Ok(parsed) =
            serde_json::from_str::<std::collections::HashMap<String, NpmOutdatedEntry>>(&stdout)
        {
            for (name, info) in parsed {
                packages.push(Package {
                    name,
                    version: info.current.unwrap_or_default(),
                    available_version: info.latest,
                    description: String::new(),
                    source: PackageSource::Npm,
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
        let status = Command::new("npm")
            .args(["install", "-g", name])
            .status()
            .await
            .context("Failed to install npm package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install npm package {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("npm")
            .args(["uninstall", "-g", name])
            .status()
            .await
            .context("Failed to remove npm package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove npm package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("npm")
            .args(["update", "-g", name])
            .status()
            .await
            .context("Failed to update npm package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update npm package {}", name)
        }
    }
}
