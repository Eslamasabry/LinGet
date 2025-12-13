use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct SnapBackend;

impl SnapBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SnapBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for SnapBackend {
    fn is_available() -> bool {
        which::which("snap").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("snap")
            .args(["list"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list snap packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        // Skip header line
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let version = parts[1].to_string();

                // Skip base snaps and snapd itself (they're system components)
                if name == "bare" || name == "snapd" || name.starts_with("core")
                    || name.starts_with("gnome-") || name.starts_with("gtk-")
                    || name.starts_with("mesa-") {
                    continue;
                }

                // Skip fetching descriptions individually - too slow
                // Descriptions can be fetched on demand when viewing details
                let description = String::new();

                packages.push(Package {
                    name,
                    version,
                    available_version: None,
                    description,
                    source: PackageSource::Snap,
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
        let output = Command::new("snap")
            .args(["refresh", "--list"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check snap updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        // Skip header line if present
        for line in stdout.lines() {
            // Skip header and empty lines
            if line.starts_with("Name") || line.is_empty() || line.contains("All snaps up to date") {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let new_version = parts[1].to_string();

                packages.push(Package {
                    name,
                    version: String::new(),
                    available_version: Some(new_version),
                    description: String::new(),
                    source: PackageSource::Snap,
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
        let status = Command::new("snap")
            .args(["install", name])
            .status()
            .await
            .context("Failed to install snap")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install snap {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("snap")
            .args(["remove", name])
            .status()
            .await
            .context("Failed to remove snap")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove snap {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("snap")
            .args(["refresh", name])
            .status()
            .await
            .context("Failed to update snap")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update snap {}", name)
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("snap")
            .args(["find", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search snaps")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        // Skip header line
        for line in stdout.lines().skip(1).take(50) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                let version = parts[1].to_string();
                // Description is the rest after publisher and notes
                let description = if parts.len() > 4 {
                    parts[4..].join(" ")
                } else {
                    String::new()
                };

                packages.push(Package {
                    name,
                    version,
                    available_version: None,
                    description,
                    source: PackageSource::Snap,
                    status: PackageStatus::NotInstalled,
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
}
