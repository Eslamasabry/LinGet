use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct CondaBackend;

impl CondaBackend {
    pub fn new() -> Self {
        Self
    }

    async fn conda_json(args: &[&str]) -> Result<serde_json::Value> {
        let output = Command::new("conda")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute conda")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).context("Failed to parse conda json")
    }

    async fn list_json() -> Result<Vec<serde_json::Value>> {
        // Prefer base env; fall back to current env.
        if let Ok(v) = Self::conda_json(&["list", "-n", "base", "--json"]).await {
            if let Some(arr) = v.as_array() {
                return Ok(arr.clone());
            }
        }
        let v = Self::conda_json(&["list", "--json"]).await?;
        Ok(v.as_array().cloned().unwrap_or_default())
    }
}

impl Default for CondaBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for CondaBackend {
    fn is_available() -> bool {
        which::which("conda").is_ok()
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
                source: PackageSource::Conda,
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
        let status = Command::new("conda")
            .args(["install", "-n", "base", "-y", name])
            .status()
            .await
            .context("Failed to install conda package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install conda package {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("conda")
            .args(["remove", "-n", "base", "-y", name])
            .status()
            .await
            .context("Failed to remove conda package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove conda package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("conda")
            .args(["update", "-n", "base", "-y", name])
            .status()
            .await
            .context("Failed to update conda package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update conda package {}", name)
        }
    }

    async fn downgrade_to(&self, name: &str, version: &str) -> Result<()> {
        if version.trim().is_empty() {
            anyhow::bail!("Version is required");
        }
        let spec = format!("{}={}", name, version);
        let status = Command::new("conda")
            .args(["install", "-n", "base", "-y", &spec])
            .status()
            .await
            .context("Failed to install a specific conda package version")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install {} via conda", spec)
        }
    }

    async fn search(&self, _query: &str) -> Result<Vec<Package>> {
        // Conda search is often slow and returns large results; leave Discover to other sources.
        Ok(Vec::new())
    }
}
