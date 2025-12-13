use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

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
        let output = Command::new("brew")
            .args(["list", "--versions"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list brew packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            let name = parts[0].to_string();
            let version = parts.last().unwrap_or(&"").to_string();

            packages.push(Package {
                name,
                version,
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
            });
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        // Prefer JSON output if available.
        let output = Command::new("brew")
            .args(["outdated", "--json=v2"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check brew updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            let mut packages = Vec::new();
            let formulae = json
                .get("formulae")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for item in formulae {
                let Some(name) = item.get("name").and_then(|v| v.as_str()) else {
                    continue;
                };
                let installed = item
                    .get("installed_versions")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let current = item
                    .get("current_version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if current.is_empty() {
                    continue;
                }

                packages.push(Package {
                    name: name.to_string(),
                    version: installed,
                    available_version: Some(current),
                    description: String::new(),
                    source: PackageSource::Brew,
                    status: PackageStatus::UpdateAvailable,
                    size: None,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                });
            }
            return Ok(packages);
        }

        Ok(Vec::new())
    }

    async fn install(&self, name: &str) -> Result<()> {
        let status = Command::new("brew")
            .args(["install", name])
            .status()
            .await
            .context("Failed to install brew package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install brew package {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("brew")
            .args(["uninstall", name])
            .status()
            .await
            .context("Failed to remove brew package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove brew package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("brew")
            .args(["upgrade", name])
            .status()
            .await
            .context("Failed to update brew package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update brew package {}", name)
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("brew")
            .args(["search", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search brew packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for name in stdout.lines().filter(|l| !l.trim().is_empty()) {
            packages.push(Package {
                name: name.trim().to_string(),
                version: String::new(),
                available_version: None,
                description: String::new(),
                source: PackageSource::Brew,
                status: PackageStatus::NotInstalled,
                size: None,
                homepage: None,
                license: None,
                maintainer: None,
                dependencies: Vec::new(),
                install_date: None,
            });
            if packages.len() >= 50 {
                break;
            }
        }

        Ok(packages)
    }
}
