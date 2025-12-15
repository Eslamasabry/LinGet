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

        // crates.io requires a User-Agent header
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("linget (https://github.com/linget/linget)")
            .build()
            .context("Failed to create HTTP client")?;

        let mut packages_with_updates = Vec::new();

        // Check each package against crates.io API
        for pkg in installed {
            let url = format!("https://crates.io/api/v1/crates/{}", pkg.name);
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(latest_version) = json
                            .get("crate")
                            .and_then(|c| c.get("max_version"))
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
                                    source: PackageSource::Cargo,
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
                    // Skip packages that fail to fetch - might be yanked or renamed
                    tracing::debug!("Failed to check updates for cargo crate: {}", pkg.name);
                }
            }
        }

        Ok(packages_with_updates)
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
                enrichment: None,
            });
        }

        Ok(packages)
    }
}
