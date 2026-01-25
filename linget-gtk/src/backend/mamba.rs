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
                update_category: None,
                enrichment: None,
            });
        }
        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        // Get currently installed packages with their versions
        let installed = self.list_installed().await.unwrap_or_default();
        let installed_map: std::collections::HashMap<String, String> = installed
            .iter()
            .map(|p| (p.name.clone(), p.version.clone()))
            .collect();

        // Run dry-run update to see what would be updated
        let output = Command::new("mamba")
            .args(["update", "-n", "base", "--all", "--dry-run", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        let Ok(output) = output else {
            tracing::warn!("Failed to check mamba updates");
            return Ok(Vec::new());
        };

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = match serde_json::from_str(&stdout) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut packages = Vec::new();

        // Parse "actions" -> "LINK" array for packages to be updated
        if let Some(actions) = json.get("actions") {
            if let Some(link) = actions.get("LINK").and_then(|v| v.as_array()) {
                for item in link {
                    let Some(name) = item.get("name").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    let new_version = item
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // Only include if we have the package installed with a different version
                    if let Some(current_version) = installed_map.get(name) {
                        if current_version != &new_version && !new_version.is_empty() {
                            packages.push(Package {
                                name: name.to_string(),
                                version: current_version.clone(),
                                available_version: Some(new_version),
                                description: String::new(),
                                source: PackageSource::Mamba,
                                status: PackageStatus::UpdateAvailable,
                                size: None,
                                homepage: None,
                                license: None,
                                maintainer: None,
                                dependencies: Vec::new(),
                                install_date: None,
                                update_category: None,
                                enrichment: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(packages)
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

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("mamba")
            .args(["search", "--json", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search mamba packages")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value =
            serde_json::from_str(&stdout).unwrap_or(serde_json::Value::Null);

        let mut packages = Vec::new();

        if let Some(obj) = json.as_object() {
            for (name, versions) in obj {
                if let Some(arr) = versions.as_array() {
                    if let Some(latest) = arr.last() {
                        let version = latest
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let channel = latest
                            .get("channel")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        packages.push(Package {
                            name: name.clone(),
                            version,
                            available_version: None,
                            description: channel,
                            source: PackageSource::Mamba,
                            status: PackageStatus::NotInstalled,
                            size: None,
                            homepage: None,
                            license: None,
                            maintainer: None,
                            dependencies: Vec::new(),
                            install_date: None,
                            update_category: None,
                            enrichment: None,
                        });
                    }
                }
            }
        }

        Ok(packages)
    }

    fn source(&self) -> PackageSource {
        PackageSource::Mamba
    }
}
