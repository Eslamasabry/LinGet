use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct AptBackend;

impl AptBackend {
    pub fn new() -> Self {
        Self
    }

    async fn run_dpkg_query(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("dpkg-query")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute dpkg-query command")?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl Default for AptBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for AptBackend {
    fn is_available() -> bool {
        which::which("apt").is_ok() && which::which("dpkg-query").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = self
            .run_dpkg_query(&[
                "-W",
                "--showformat=${Package}\\t${Version}\\t${Description}\\n",
            ])
            .await?;

        let mut packages = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.splitn(3, '\t').collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let version = parts[1].to_string();
                let description = parts.get(2).unwrap_or(&"").to_string();

                // Take only the first line of description
                let description = description.lines().next().unwrap_or("").to_string();

                packages.push(Package {
                    name,
                    version,
                    available_version: None,
                    description,
                    source: PackageSource::Apt,
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
        // First, simulate an update to get the list
        let output = Command::new("apt")
            .args(["list", "--upgradable"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check for updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines().skip(1) {
            // Skip "Listing..." header
            // Format: package/source version arch [upgradable from: old_version]
            if let Some(name) = line.split('/').next() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let new_version = parts.get(1).unwrap_or(&"").to_string();
                    let old_version = line
                        .split("from: ")
                        .nth(1)
                        .map(|s| s.trim_end_matches(']').to_string())
                        .unwrap_or_default();

                    packages.push(Package {
                        name: name.to_string(),
                        version: old_version,
                        available_version: Some(new_version),
                        description: String::new(),
                        source: PackageSource::Apt,
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
        }

        Ok(packages)
    }

    async fn install(&self, name: &str) -> Result<()> {
        let status = Command::new("pkexec")
            .args(["apt", "install", "-y", name])
            .status()
            .await
            .context("Failed to install package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install package {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("pkexec")
            .args(["apt", "remove", "-y", name])
            .status()
            .await
            .context("Failed to remove package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("pkexec")
            .args(["apt", "install", "--only-upgrade", "-y", name])
            .status()
            .await
            .context("Failed to update package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update package {}", name)
        }
    }
}
