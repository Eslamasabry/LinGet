use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus, Repository};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

/// Parse human-readable size strings like "1.2 GB", "500 MB", "100 kB"
fn parse_human_size(s: &str) -> Option<u64> {
    let s = s.trim();
    let mut num_end = 0;
    for (i, c) in s.char_indices() {
        if c.is_ascii_digit() || c == '.' {
            num_end = i + c.len_utf8();
        } else if !c.is_whitespace() {
            break;
        }
    }
    let num: f64 = s[..num_end].trim().parse().ok()?;
    let unit = s[num_end..].trim().to_lowercase();
    let multiplier: u64 = match unit.as_str() {
        "b" | "bytes" => 1,
        "kb" | "kib" => 1024,
        "mb" | "mib" => 1024 * 1024,
        "gb" | "gib" => 1024 * 1024 * 1024,
        "tb" | "tib" => 1024 * 1024 * 1024 * 1024,
        _ => return None,
    };
    Some((num * multiplier as f64) as u64)
}

pub struct FlatpakBackend;

impl FlatpakBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FlatpakBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for FlatpakBackend {
    fn is_available() -> bool {
        which::which("flatpak").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        // Include size column (returns bytes)
        let output = Command::new("flatpak")
            .args(["list", "--app", "--columns=application,version,name,size"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list flatpak apps")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let app_id = parts[0].to_string();
                let version = parts[1].to_string();
                let name = parts[2].to_string();
                // Parse size (flatpak returns human-readable like "1.2 GB")
                let size = parts.get(3).and_then(|s| parse_human_size(s));

                packages.push(Package {
                    name: app_id,
                    version,
                    available_version: None,
                    description: name,
                    source: PackageSource::Flatpak,
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
        let output = Command::new("flatpak")
            .args([
                "remote-ls",
                "--updates",
                "--app",
                "--columns=application,version,name",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check flatpak updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let app_id = parts[0].to_string();
                let new_version = parts[1].to_string();
                let name = parts[2].to_string();

                packages.push(Package {
                    name: app_id,
                    version: String::new(),
                    available_version: Some(new_version),
                    description: name,
                    source: PackageSource::Flatpak,
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
        let status = Command::new("flatpak")
            .args(["install", "-y", name])
            .status()
            .await
            .context("Failed to install flatpak")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install flatpak {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["uninstall", "-y", name])
            .status()
            .await
            .context("Failed to remove flatpak")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove flatpak {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["update", "-y", name])
            .status()
            .await
            .context("Failed to update flatpak")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update flatpak {}", name)
        }
    }

    async fn list_repositories(&self) -> Result<Vec<Repository>> {
        // flatpak remotes lists all configured remotes
        let output = Command::new("flatpak")
            .args(["remotes", "--columns=name,url,options"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list flatpak remotes")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut repos = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if !parts.is_empty() && !parts[0].is_empty() {
                let name = parts[0].to_string();
                let url = parts.get(1).map(|s| s.to_string()).filter(|s| !s.is_empty());
                let options = parts.get(2).unwrap_or(&"");
                // Check if disabled by looking at options
                let enabled = !options.contains("disabled");

                repos.push(Repository::new(name, PackageSource::Flatpak, enabled, url));
            }
        }

        Ok(repos)
    }

    async fn add_repository(&self, url: &str, name: Option<&str>) -> Result<()> {
        // flatpak remote-add <name> <url>
        let repo_name = name.unwrap_or("custom");
        let status = Command::new("flatpak")
            .args(["remote-add", "--if-not-exists", repo_name, url])
            .status()
            .await
            .context("Failed to add flatpak remote")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to add flatpak remote {}", url)
        }
    }

    async fn remove_repository(&self, name: &str) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["remote-delete", "--force", name])
            .status()
            .await
            .context("Failed to remove flatpak remote")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove flatpak remote {}", name)
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("flatpak")
            .args([
                "search",
                query,
                "--columns=application,version,name,description",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search flatpak")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: parts.get(1).unwrap_or(&"").to_string(),
                    available_version: None,
                    description: parts.get(2).unwrap_or(&"").to_string(),
                    source: PackageSource::Flatpak,
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
