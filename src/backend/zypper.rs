use super::PackageBackend;
use super::{run_pkexec, Suggest};
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct ZypperBackend;

impl ZypperBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ZypperBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for ZypperBackend {
    fn is_available() -> bool {
        which::which("zypper").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        // Use rpm as a stable way to list installed packages on RPM-based distros.
        if which::which("rpm").is_err() {
            return Ok(Vec::new());
        }

        let output = Command::new("rpm")
            .args(["-qa", "--qf", "%{NAME}\t%{VERSION}-%{RELEASE}\n"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list installed rpm packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let mut parts = line.split('\t');
            let Some(name) = parts.next() else { continue };
            let version = parts.next().unwrap_or("").to_string();
            if name.trim().is_empty() {
                continue;
            }
            packages.push(Package {
                name: name.to_string(),
                version,
                available_version: None,
                description: String::new(),
                source: PackageSource::Zypper,
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
        let output = Command::new("zypper")
            .args(["--non-interactive", "--quiet", "lu"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check zypper updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let cols: Vec<String> = line.split('|').map(|s| s.trim().to_string()).collect();
            if cols.len() < 6 {
                continue;
            }
            if cols[0].starts_with("S") || cols[2].eq_ignore_ascii_case("Name") {
                continue;
            }

            let name = cols[2].clone();
            let current = cols[3].clone();
            let available = cols[4].clone();
            if name.is_empty() || available.is_empty() {
                continue;
            }

            packages.push(Package {
                name,
                version: current,
                available_version: Some(available),
                description: String::new(),
                source: PackageSource::Zypper,
                status: PackageStatus::UpdateAvailable,
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

    async fn install(&self, name: &str) -> Result<()> {
        run_pkexec(
            "zypper",
            &["--non-interactive", "install", "-y", "--", name],
            &format!("Failed to install zypper package {}", name),
            Suggest {
                command: format!("sudo zypper --non-interactive install -y -- {}", name),
            },
        )
        .await
    }

    async fn remove(&self, name: &str) -> Result<()> {
        run_pkexec(
            "zypper",
            &["--non-interactive", "remove", "-y", "--", name],
            &format!("Failed to remove zypper package {}", name),
            Suggest {
                command: format!("sudo zypper --non-interactive remove -y -- {}", name),
            },
        )
        .await
    }

    async fn update(&self, name: &str) -> Result<()> {
        run_pkexec(
            "zypper",
            &["--non-interactive", "update", "-y", "--", name],
            &format!("Failed to update zypper package {}", name),
            Suggest {
                command: format!("sudo zypper --non-interactive update -y -- {}", name),
            },
        )
        .await
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("zypper")
            .args(["--non-interactive", "--quiet", "se", "-s", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search zypper packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let cols: Vec<String> = line.split('|').map(|s| s.trim().to_string()).collect();
            if cols.len() < 4 {
                continue;
            }
            if cols[0].starts_with("S") || cols[1].eq_ignore_ascii_case("Name") {
                continue;
            }

            let name = cols[1].clone();
            let version = cols.get(2).cloned().unwrap_or_default();
            let description = cols.get(3).cloned().unwrap_or_default();
            if name.is_empty() {
                continue;
            }

            packages.push(Package {
                name,
                version,
                available_version: None,
                description,
                source: PackageSource::Zypper,
                status: PackageStatus::NotInstalled,
                size: None,
                homepage: None,
                license: None,
                maintainer: None,
                dependencies: Vec::new(),
                install_date: None,
            });

            if packages.len() >= 50 {
                break;
            }
        }

        Ok(packages)
    }
}
