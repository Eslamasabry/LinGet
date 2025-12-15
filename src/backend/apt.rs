use super::PackageBackend;
use super::{run_pkexec, Suggest};
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct AptBackend;

impl AptBackend {
    pub fn new() -> Self {
        Self
    }

    async fn run_dpkg_query(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("dpkg-query")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute dpkg-query command")?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl Default for AptBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for AptBackend {
    fn is_available() -> bool {
        which::which("apt").is_ok() && which::which("dpkg-query").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        // Include Installed-Size (in KB) in the query
        let output = self
            .run_dpkg_query(&[
                "-W",
                "--showformat=${Package}\\t${Version}\\t${Installed-Size}\\t${Description}\\n",
            ])
            .await?;

        let mut packages = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.splitn(4, '\t').collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let version = parts[1].to_string();
                // Size is in KB, convert to bytes
                let size = parts
                    .get(2)
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(|kb| kb * 1024);
                let description = parts.get(3).unwrap_or(&"").to_string();

                // Take only the first line of description
                let description = description.lines().next().unwrap_or("").to_string();

                packages.push(Package {
                    name,
                    version,
                    available_version: None,
                    description,
                    source: PackageSource::Apt,
                    status: PackageStatus::Installed,
                    size,
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
        // First, simulate an update to get the list
        let output = Command::new("apt")
            .args(["list", "--upgradable"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check for updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines().skip(1) {
            // Skip "Listing..." header
            // Format: package/source version arch [upgradable from: old_version]
            if let Some(name) = line.split('/').next() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let new_version = parts.get(1).unwrap_or(&"").to_string();
                    let old_version = line
                        .split("from: ")
                        .nth(1)
                        .map(|s| s.trim_end_matches(']').to_string())
                        .unwrap_or_default();

                    packages.push(Package {
                        name: name.to_string(),
                        version: old_version,
                        available_version: Some(new_version),
                        description: String::new(),
                        source: PackageSource::Apt,
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

        Ok(packages)
    }

    async fn install(&self, name: &str) -> Result<()> {
        run_pkexec(
            "apt",
            &["install", "-y", "--", name],
            &format!("Failed to install package {}", name),
            Suggest {
                command: format!("sudo apt install -y -- {}", name),
            },
        )
        .await
    }

    async fn remove(&self, name: &str) -> Result<()> {
        run_pkexec(
            "apt",
            &["remove", "-y", "--", name],
            &format!("Failed to remove package {}", name),
            Suggest {
                command: format!("sudo apt remove -y -- {}", name),
            },
        )
        .await
    }

    async fn update(&self, name: &str) -> Result<()> {
        run_pkexec(
            "apt",
            &["install", "--only-upgrade", "-y", "--", name],
            &format!("Failed to update package {}", name),
            Suggest {
                command: format!("sudo apt install --only-upgrade -y -- {}", name),
            },
        )
        .await
    }

    async fn available_downgrade_versions(&self, name: &str) -> Result<Vec<String>> {
        // `apt-cache madison <pkg>` output:
        //  pkg | 1.2.3-1 | http://... focal/main amd64 Packages
        let output = Command::new("apt-cache")
            .args(["madison", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list package versions")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut versions = Vec::new();
        for line in stdout.lines() {
            let cols: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
            if cols.len() >= 2 && !cols[1].is_empty() {
                versions.push(cols[1].to_string());
            }
        }
        versions.sort();
        versions.dedup();
        versions.reverse(); // newest first
        Ok(versions)
    }

    async fn downgrade_to(&self, name: &str, version: &str) -> Result<()> {
        let target = format!("{}={}", name, version);
        run_pkexec(
            "apt",
            &["install", "-y", "--allow-downgrades", "--", &target],
            &format!("Failed to downgrade package {}", name),
            Suggest {
                command: format!("sudo apt install -y --allow-downgrades -- {}", target),
            },
        )
        .await
    }

    async fn get_changelog(&self, name: &str) -> Result<Option<String>> {
        // apt-get changelog fetches the Debian changelog for the package
        let output = Command::new("apt-get")
            .args(["changelog", "--", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get changelog")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.is_empty() {
                return Ok(None);
            }
            // Convert Debian changelog format to markdown-ish format
            let changelog = stdout
                .lines()
                .take(500) // Limit to reasonable size
                .collect::<Vec<_>>()
                .join("\n");
            Ok(Some(format!("```\n{}\n```", changelog)))
        } else {
            Ok(None)
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("apt-cache")
            .args(["search", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines().take(50) {
            // Limit results
            let parts: Vec<&str> = line.splitn(2, " - ").collect();
            if parts.len() == 2 {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: String::new(),
                    available_version: None,
                    description: parts[1].to_string(),
                    source: PackageSource::Apt,
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
