use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct CargoBackend;

impl CargoBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CargoBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for CargoBackend {
    fn is_available() -> bool {
        which::which("cargo").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("cargo")
            .args(["install", "--list"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list cargo installed crates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            // Example: ripgrep v13.0.0:
            if !line.ends_with(':') {
                continue;
            }
            let header = line.trim_end_matches(':').trim();
            let Some((name, version_part)) = header.split_once(' ') else {
                continue;
            };
            let version = version_part.trim_start_matches('v').to_string();
            if name.is_empty() {
                continue;
            }

            packages.push(Package {
                name: name.to_string(),
                version,
                available_version: None,
                description: String::new(),
                source: PackageSource::Cargo,
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
        // There isn't a built-in, reliable "outdated" command for cargo without extra tooling.
        Ok(Vec::new())
    }

    async fn install(&self, name: &str) -> Result<()> {
        let status = Command::new("cargo")
            .args(["install", name])
            .status()
            .await
            .context("Failed to install cargo crate")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install cargo crate {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("cargo")
            .args(["uninstall", name])
            .status()
            .await
            .context("Failed to uninstall cargo crate")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to uninstall cargo crate {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        // Best-effort: reinstall with --force to pull latest.
        let status = Command::new("cargo")
            .args(["install", name, "--force"])
            .status()
            .await
            .context("Failed to update cargo crate")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update cargo crate {}", name)
        }
    }

    async fn downgrade_to(&self, name: &str, version: &str) -> Result<()> {
        let status = Command::new("cargo")
            .args(["install", name, "--version", version, "--force"])
            .status()
            .await
            .context("Failed to install a specific cargo crate version")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install {} v{} via cargo", name, version)
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("cargo")
            .args(["search", query, "--limit", "50"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search cargo crates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            // Example: ripgrep = "13.0.0"    # Recursively searches directories...
            let Some((name, rest)) = line.split_once(" = ") else {
                continue;
            };
            let version = rest.split('"').nth(1).unwrap_or("").to_string();
            let description = rest
                .split('#')
                .nth(1)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            if name.trim().is_empty() {
                continue;
            }

            packages.push(Package {
                name: name.trim().to_string(),
                version,
                available_version: None,
                description,
                source: PackageSource::Cargo,
                status: PackageStatus::NotInstalled,
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
}
