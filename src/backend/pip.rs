use super::PackageBackend;
use crate::backend::SUGGEST_PREFIX;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

pub struct PipBackend;

impl PipBackend {
    pub fn new() -> Self {
        Self
    }

    fn get_pip_command() -> &'static str {
        // Try pip3 first, then pip
        if which::which("pip3").is_ok() {
            "pip3"
        } else {
            "pip"
        }
    }
}

impl Default for PipBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct PipPackageInfo {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct PipOutdatedInfo {
    name: String,
    version: String,
    latest_version: String,
}

#[async_trait]
impl PackageBackend for PipBackend {
    fn is_available() -> bool {
        which::which("pip3").is_ok() || which::which("pip").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let pip = Self::get_pip_command();
        let output = Command::new(pip)
            .args(["list", "--format=json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list pip packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        if let Ok(parsed) = serde_json::from_str::<Vec<PipPackageInfo>>(&stdout) {
            for pkg in parsed {
                packages.push(Package {
                    name: pkg.name,
                    version: pkg.version,
                    available_version: None,
                    description: String::new(),
                    source: PackageSource::Pip,
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
        let pip = Self::get_pip_command();
        let output = Command::new(pip)
            .args(["list", "--outdated", "--format=json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check pip updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        if let Ok(parsed) = serde_json::from_str::<Vec<PipOutdatedInfo>>(&stdout) {
            for pkg in parsed {
                packages.push(Package {
                    name: pkg.name,
                    version: pkg.version,
                    available_version: Some(pkg.latest_version),
                    description: String::new(),
                    source: PackageSource::Pip,
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

        Ok(packages)
    }

    async fn install(&self, name: &str) -> Result<()> {
        let pip = Self::get_pip_command();
        let status = Command::new(pip)
            .args(["install", "--user", name])
            .status()
            .await
            .context("Failed to install pip package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install pip package {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let pip = Self::get_pip_command();
        let status = Command::new(pip)
            .args(["uninstall", "-y", name])
            .status()
            .await
            .context("Failed to remove pip package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove pip package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let pip = Self::get_pip_command();
        let status = Command::new(pip)
            .args(["install", "--user", "--upgrade", name])
            .status()
            .await
            .context("Failed to update pip package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update pip package {}", name)
        }
    }

    async fn downgrade_to(&self, name: &str, version: &str) -> Result<()> {
        let pip = Self::get_pip_command();
        let spec = format!("{}=={}", name, version);
        let output = Command::new(pip)
            .args(["install", "--user", &spec])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to install a specific pip package version")?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let lowered = stderr.to_lowercase();
        if lowered.contains("can not perform a '--user' install")
            || lowered.contains("user site-packages are not visible")
        {
            anyhow::bail!(
                "pip rejected a --user install (likely a virtualenv).\n\n{} {} install {}\n",
                SUGGEST_PREFIX,
                pip,
                spec
            );
        }

        anyhow::bail!("Failed to install {}: {}", spec, stderr);
    }

    async fn available_downgrade_versions(&self, name: &str) -> Result<Vec<String>> {
        let pip = Self::get_pip_command();
        let output = Command::new(pip)
            .args(["index", "versions", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        let Ok(output) = output else {
            return Ok(Vec::new());
        };
        if !output.status.success() {
            return Ok(Vec::new());
        }

        // Output example:
        //   <name> (<installed>) - Available versions: x, y, z
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut versions: Vec<String> = Vec::new();
        for line in stdout.lines() {
            let Some((_, tail)) = line.split_once("Available versions:") else {
                continue;
            };
            for v in tail.split(',') {
                let v = v.trim();
                if !v.is_empty() {
                    versions.push(v.to_string());
                }
            }
        }

        Ok(versions)
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        // pip search is disabled on PyPI, so we use pip index versions as a workaround
        // This only works if you know the exact package name
        // For now, return empty results - user should use PyPI website for searching
        tracing::info!("pip search is not available - PyPI disabled this feature");

        // Try to get info about the exact package name
        let pip = Self::get_pip_command();
        let output = Command::new(pip)
            .args(["show", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        let mut packages = Vec::new();

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut name = String::new();
                let mut version = String::new();
                let mut description = String::new();

                for line in stdout.lines() {
                    if let Some(value) = line.strip_prefix("Name: ") {
                        name = value.to_string();
                    } else if let Some(value) = line.strip_prefix("Version: ") {
                        version = value.to_string();
                    } else if let Some(value) = line.strip_prefix("Summary: ") {
                        description = value.to_string();
                    }
                }

                if !name.is_empty() {
                    packages.push(Package {
                        name,
                        version,
                        available_version: None,
                        description,
                        source: PackageSource::Pip,
                        status: PackageStatus::NotInstalled,
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
}
