use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct DnfBackend;

impl DnfBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DnfBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for DnfBackend {
    fn is_available() -> bool {
        which::which("dnf").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        // dnf repoquery --installed --queryformat "%{NAME}|%{VERSION}|%{SUMMARY}"
        let output = Command::new("dnf")
            .args([
                "repoquery",
                "--installed",
                "--queryformat",
                "%{NAME}|%{VERSION}|%{SUMMARY}",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list installed dnf packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 3 {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    available_version: None,
                    description: parts[2].to_string(),
                    source: PackageSource::Dnf,
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
        // dnf check-update
        // Output format is roughly:
        // package.arch  version  repo
        // Returns exit code 100 if updates available
        let output = Command::new("dnf")
            .arg("check-update")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check dnf updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        // Skip potential header lines or system messages until we see package list
        let mut in_list = false;
        for line in stdout.lines() {
            if line.trim().is_empty() {
                in_list = true; // Often an empty line separates headers
                continue;
            }
            // Heuristic: lines with 3 columns are likely packages
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                // Ignore "Security:" or "Obsoleting" lines
                if parts[0].ends_with(':') {
                    continue;
                }
                
                // dnf output often includes architecture in name (e.g. package.x86_64)
                let name_arch = parts[0];
                let name = name_arch.split('.').next().unwrap_or(name_arch).to_string();
                let version = parts[1].to_string();

                packages.push(Package {
                    name,
                    version: String::new(), // We don't know current version easily here
                    available_version: Some(version),
                    description: String::new(),
                    source: PackageSource::Dnf,
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
        let status = Command::new("pkexec")
            .args(["dnf", "install", "-y", name])
            .status()
            .await
            .context("Failed to install dnf package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install dnf package {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("pkexec")
            .args(["dnf", "remove", "-y", name])
            .status()
            .await
            .context("Failed to remove dnf package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove dnf package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("pkexec")
            .args(["dnf", "update", "-y", name])
            .status()
            .await
            .context("Failed to update dnf package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update dnf package {}", name)
        }
    }
}
