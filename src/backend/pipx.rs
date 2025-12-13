use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct PipxBackend;

impl PipxBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PipxBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for PipxBackend {
    fn is_available() -> bool {
        which::which("pipx").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("pipx")
            .args(["list", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list pipx packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value =
            serde_json::from_str(&stdout).context("Failed to parse pipx json")?;
        let venvs = json
            .get("venvs")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let mut packages = Vec::new();
        for (name, v) in venvs {
            let version = v
                .get("metadata")
                .and_then(|m| m.get("main_package"))
                .and_then(|p| p.get("package_version"))
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();

            packages.push(Package {
                name,
                version,
                available_version: None,
                description: String::new(),
                source: PackageSource::Pipx,
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
        // pipx doesn't provide a reliable "list outdated" without performing upgrades.
        Ok(Vec::new())
    }

    async fn install(&self, name: &str) -> Result<()> {
        let status = Command::new("pipx")
            .args(["install", name])
            .status()
            .await
            .context("Failed to install pipx package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install pipx package {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("pipx")
            .args(["uninstall", name])
            .status()
            .await
            .context("Failed to uninstall pipx package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to uninstall pipx package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("pipx")
            .args(["upgrade", name])
            .status()
            .await
            .context("Failed to update pipx package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update pipx package {}", name)
        }
    }

    async fn downgrade_to(&self, name: &str, version: &str) -> Result<()> {
        let spec = format!("{}=={}", name, version);
        let status = Command::new("pipx")
            .args(["install", "--force", &spec])
            .status()
            .await
            .context("Failed to install a specific pipx package version")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install {} via pipx", spec)
        }
    }

    async fn search(&self, _query: &str) -> Result<Vec<Package>> {
        // pipx doesn't have a search command.
        Ok(Vec::new())
    }
}
