use super::PackageBackend;
use crate::backend::SUGGEST_PREFIX;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

pub struct NpmBackend;

impl NpmBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NpmBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct NpmListOutput {
    dependencies: Option<std::collections::HashMap<String, NpmPackageInfo>>,
}

#[derive(Debug, Deserialize)]
struct NpmPackageInfo {
    version: Option<String>,
    _resolved: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NpmOutdatedEntry {
    current: Option<String>,
    _wanted: Option<String>,
    latest: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NpmVersions {
    One(String),
    Many(Vec<String>),
}

#[async_trait]
impl PackageBackend for NpmBackend {
    fn is_available() -> bool {
        which::which("npm").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("npm")
            .args(["list", "-g", "--depth=0", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list npm packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        if let Ok(parsed) = serde_json::from_str::<NpmListOutput>(&stdout) {
            if let Some(deps) = parsed.dependencies {
                for (name, info) in deps {
                    packages.push(Package {
                        name,
                        version: info.version.unwrap_or_default(),
                        available_version: None,
                        description: String::new(),
                        source: PackageSource::Npm,
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
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        let output = Command::new("npm")
            .args(["outdated", "-g", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check npm updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        if let Ok(parsed) =
            serde_json::from_str::<std::collections::HashMap<String, NpmOutdatedEntry>>(&stdout)
        {
            for (name, info) in parsed {
                packages.push(Package {
                    name,
                    version: info.current.unwrap_or_default(),
                    available_version: info.latest,
                    description: String::new(),
                    source: PackageSource::Npm,
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
        let status = Command::new("npm")
            .args(["install", "-g", name])
            .status()
            .await
            .context("Failed to install npm package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install npm package {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("npm")
            .args(["uninstall", "-g", name])
            .status()
            .await
            .context("Failed to remove npm package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove npm package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("npm")
            .args(["update", "-g", name])
            .status()
            .await
            .context("Failed to update npm package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update npm package {}", name)
        }
    }

    async fn downgrade_to(&self, name: &str, version: &str) -> Result<()> {
        let spec = format!("{}@{}", name, version);
        let output = Command::new("npm")
            .args(["install", "-g", &spec])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to install a specific npm package version")?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let lowered = stderr.to_lowercase();
        if lowered.contains("eacces")
            || lowered.contains("permission")
            || lowered.contains("access")
        {
            anyhow::bail!(
                "Failed to install {}.\n\n{} sudo npm install -g {}\n",
                spec,
                SUGGEST_PREFIX,
                spec
            );
        }

        anyhow::bail!("Failed to install {}: {}", spec, stderr);
    }

    async fn available_downgrade_versions(&self, name: &str) -> Result<Vec<String>> {
        let output = Command::new("npm")
            .args(["view", name, "versions", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to query npm versions")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed = serde_json::from_str::<NpmVersions>(&stdout).ok();
        let mut versions = match parsed {
            Some(NpmVersions::One(v)) => vec![v],
            Some(NpmVersions::Many(v)) => v,
            None => Vec::new(),
        };

        // Present newest first.
        versions.reverse();
        Ok(versions)
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("npm")
            .args(["search", query, "--json", "--long"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search npm packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        #[derive(Deserialize)]
        struct NpmSearchResult {
            name: String,
            version: Option<String>,
            description: Option<String>,
        }

        if let Ok(results) = serde_json::from_str::<Vec<NpmSearchResult>>(&stdout) {
            for result in results.into_iter().take(50) {
                packages.push(Package {
                    name: result.name,
                    version: result.version.unwrap_or_default(),
                    available_version: None,
                    description: result.description.unwrap_or_default(),
                    source: PackageSource::Npm,
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
