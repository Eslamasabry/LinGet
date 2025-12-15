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

    /// Simple semver comparison - returns true if new_ver > old_ver
    fn is_newer_version(new_ver: &str, old_ver: &str) -> bool {
        let parse_version = |s: &str| -> Vec<u64> {
            s.split(['.', '-', '+'])
                .filter_map(|p| p.parse::<u64>().ok())
                .collect()
        };

        let new_parts = parse_version(new_ver);
        let old_parts = parse_version(old_ver);

        for i in 0..new_parts.len().max(old_parts.len()) {
            let new_part = new_parts.get(i).copied().unwrap_or(0);
            let old_part = old_parts.get(i).copied().unwrap_or(0);
            if new_part > old_part {
                return true;
            } else if new_part < old_part {
                return false;
            }
        }
        false
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
                enrichment: None,
            });
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        let installed = self.list_installed().await.unwrap_or_default();
        if installed.is_empty() {
            return Ok(Vec::new());
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        let mut packages_with_updates = Vec::new();

        // Check each package against PyPI API
        for pkg in installed {
            let url = format!("https://pypi.org/pypi/{}/json", pkg.name);
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(latest_version) = json
                            .get("info")
                            .and_then(|i| i.get("version"))
                            .and_then(|v| v.as_str())
                        {
                            // Compare versions - if different and newer, there's an update
                            if !pkg.version.is_empty()
                                && latest_version != pkg.version
                                && Self::is_newer_version(latest_version, &pkg.version)
                            {
                                packages_with_updates.push(Package {
                                    name: pkg.name,
                                    version: pkg.version,
                                    available_version: Some(latest_version.to_string()),
                                    description: String::new(),
                                    source: PackageSource::Pipx,
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
                    }
                }
                _ => {
                    // Skip packages that fail to fetch - might be private or renamed
                    tracing::debug!("Failed to check updates for pipx package: {}", pkg.name);
                }
            }
        }

        Ok(packages_with_updates)
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
