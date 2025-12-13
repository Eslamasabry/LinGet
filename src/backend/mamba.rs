use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct MambaBackend;

impl MambaBackend {
    pub fn new() -> Self {
        Self
    }

    async fn mamba_json(args: &[&str]) -> Result<serde_json::Value> {
        let output = Command::new("mamba")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute mamba")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).context("Failed to parse mamba json")
    }

    async fn list_json() -> Result<Vec<serde_json::Value>> {
        if let Ok(v) = Self::mamba_json(&["list", "-n", "base", "--json"]).await {
            if let Some(arr) = v.as_array() {
                return Ok(arr.clone());
            }
        }
        let v = Self::mamba_json(&["list", "--json"]).await?;
        Ok(v.as_array().cloned().unwrap_or_default())
    }
}

impl Default for MambaBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for MambaBackend {
    fn is_available() -> bool {
        which::which("mamba").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let items = Self::list_json().await?;
        let mut packages = Vec::new();
        for item in items {
            let Some(name) = item.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            let version = item
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            packages.push(Package {
                name: name.to_string(),
                version,
                available_version: None,
                description: String::new(),
                source: PackageSource::Mamba,
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
        Ok(Vec::new())
    }

    async fn install(&self, name: &str) -> Result<()> {
        let status = Command::new("mamba")
            .args(["install", "-n", "base", "-y", name])
            .status()
            .await
            .context("Failed to install mamba package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install mamba package {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("mamba")
            .args(["remove", "-n", "base", "-y", name])
            .status()
            .await
            .context("Failed to remove mamba package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove mamba package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("mamba")
            .args(["update", "-n", "base", "-y", name])
            .status()
            .await
            .context("Failed to update mamba package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update mamba package {}", name)
        }
    }

    async fn downgrade_to(&self, name: &str, version: &str) -> Result<()> {
        if version.trim().is_empty() {
            anyhow::bail!("Version is required");
        }
        let spec = format!("{}={}", name, version);
        let status = Command::new("mamba")
            .args(["install", "-n", "base", "-y", &spec])
            .status()
            .await
            .context("Failed to install a specific mamba package version")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install {} via mamba", spec)
        }
    }

    async fn search(&self, _query: &str) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }
}
