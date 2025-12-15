use super::PackageBackend;
use super::{run_pkexec, Suggest};
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
                    enrichment: None,
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
        for line in stdout.lines() {
            if line.trim().is_empty() {
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
                    enrichment: None,
                });
            }
        }

        Ok(packages)
    }

    async fn install(&self, name: &str) -> Result<()> {
        run_pkexec(
            "dnf",
            &["install", "-y", "--", name],
            &format!("Failed to install dnf package {}", name),
            Suggest {
                command: format!("sudo dnf install -y -- {}", name),
            },
        )
        .await
    }

    async fn remove(&self, name: &str) -> Result<()> {
        run_pkexec(
            "dnf",
            &["remove", "-y", "--", name],
            &format!("Failed to remove dnf package {}", name),
            Suggest {
                command: format!("sudo dnf remove -y -- {}", name),
            },
        )
        .await
    }

    async fn update(&self, name: &str) -> Result<()> {
        run_pkexec(
            "dnf",
            &["update", "-y", "--", name],
            &format!("Failed to update dnf package {}", name),
            Suggest {
                command: format!("sudo dnf update -y -- {}", name),
            },
        )
        .await
    }

    async fn downgrade(&self, name: &str) -> Result<()> {
        run_pkexec(
            "dnf",
            &["downgrade", "-y", "--", name],
            &format!("Failed to downgrade dnf package {}", name),
            Suggest {
                command: format!("sudo dnf downgrade -y -- {}", name),
            },
        )
        .await
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        // dnf search query
        let output = Command::new("dnf")
            .args(["search", "-q", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search dnf packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        // Output format:
        // name.arch : summary
        for line in stdout.lines() {
            if let Some((name_part, summary)) = line.split_once(" : ") {
                let name = name_part
                    .split('.')
                    .next()
                    .unwrap_or(name_part)
                    .trim()
                    .to_string();

                packages.push(Package {
                    name,
                    version: String::new(),
                    available_version: None,
                    description: summary.trim().to_string(),
                    source: PackageSource::Dnf,
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
