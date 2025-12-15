use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct FlatpakBackend;

impl FlatpakBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FlatpakBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for FlatpakBackend {
    fn is_available() -> bool {
        which::which("flatpak").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("flatpak")
            .args(["list", "--app", "--columns=application,version,name"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list flatpak apps")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let app_id = parts[0].to_string();
                let version = parts[1].to_string();
                let name = parts[2].to_string();

                packages.push(Package {
                    name: app_id,
                    version,
                    available_version: None,
                    description: name,
                    source: PackageSource::Flatpak,
                    status: PackageStatus::Installed,
                    size: None,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                    enrichment: None,
                });
            }
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        let output = Command::new("flatpak")
            .args([
                "remote-ls",
                "--updates",
                "--app",
                "--columns=application,version,name",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check flatpak updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let app_id = parts[0].to_string();
                let new_version = parts[1].to_string();
                let name = parts[2].to_string();

                packages.push(Package {
                    name: app_id,
                    version: String::new(),
                    available_version: Some(new_version),
                    description: name,
                    source: PackageSource::Flatpak,
                    status: PackageStatus::UpdateAvailable,
                    size: None,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                    enrichment: None,
                });
            }
        }

        Ok(packages)
    }

    async fn install(&self, name: &str) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["install", "-y", name])
            .status()
            .await
            .context("Failed to install flatpak")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install flatpak {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["uninstall", "-y", name])
            .status()
            .await
            .context("Failed to remove flatpak")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove flatpak {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["update", "-y", name])
            .status()
            .await
            .context("Failed to update flatpak")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update flatpak {}", name)
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("flatpak")
            .args([
                "search",
                query,
                "--columns=application,version,name,description",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search flatpak")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: parts.get(1).unwrap_or(&"").to_string(),
                    available_version: None,
                    description: parts.get(2).unwrap_or(&"").to_string(),
                    source: PackageSource::Flatpak,
                    status: PackageStatus::NotInstalled,
                    size: None,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                    enrichment: None,
                });
            }
        }

        Ok(packages)
    }
}
