use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct AurBackend {
    helper: String,
}

impl AurBackend {
    pub fn new() -> Self {
        let helper = if which::which("paru").is_ok() {
            "paru".to_string()
        } else {
            "yay".to_string()
        };
        Self { helper }
    }
}

impl Default for AurBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for AurBackend {
    fn is_available() -> bool {
        which::which("yay").is_ok() || which::which("paru").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        // -Qm lists "foreign" packages (often AUR).
        let output = Command::new(&self.helper)
            .args(["-Qm"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .with_context(|| format!("Failed to list AUR packages via {}", self.helper))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    available_version: None,
                    description: String::new(),
                    source: PackageSource::Aur,
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
        // -Qua lists AUR updates.
        let output = Command::new(&self.helper)
            .args(["-Qua"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .with_context(|| format!("Failed to check AUR updates via {}", self.helper))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        for line in stdout.lines() {
            // name old -> new
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    available_version: Some(parts[3].to_string()),
                    description: String::new(),
                    source: PackageSource::Aur,
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
        // Use --noconfirm to skip prompts. This skips PKGBUILD review which is a security risk.
        // The user should have already reviewed the package on the AUR website.
        let status = Command::new(&self.helper)
            .args(["-S", "--noconfirm", "--needed", name])
            .status()
            .await
            .with_context(|| {
                format!("Failed to install AUR package {} via {}", name, self.helper)
            })?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install AUR package {} via {}", name, self.helper)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        // Use pacman directly for removal (doesn't need AUR helper)
        // Requires pkexec for root access
        let status = Command::new("pkexec")
            .args(["pacman", "-R", "--noconfirm", name])
            .status()
            .await
            .context("Failed to remove AUR package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove AUR package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        // Reinstall to get the latest version
        let status = Command::new(&self.helper)
            .args(["-S", "--noconfirm", name])
            .status()
            .await
            .with_context(|| {
                format!("Failed to update AUR package {} via {}", name, self.helper)
            })?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update AUR package {} via {}", name, self.helper)
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        // Best-effort: use helper's -Ss which includes repo results too.
        let output = Command::new(&self.helper)
            .args(["-Ss", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .with_context(|| format!("Failed to search AUR via {}", self.helper))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        let lines: Vec<&str> = stdout.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let header = lines[i];
            if let Some((repo_name, rest)) = header.split_once(' ') {
                if let Some((_, name)) = repo_name.split_once('/') {
                    let version = rest.split_whitespace().next().unwrap_or("").to_string();
                    let description = if i + 1 < lines.len() {
                        lines[i + 1].trim().to_string()
                    } else {
                        String::new()
                    };
                    packages.push(Package {
                        name: name.to_string(),
                        version,
                        available_version: None,
                        description,
                        source: PackageSource::Aur,
                        status: PackageStatus::NotInstalled,
                        size: None,
                        homepage: None,
                        license: None,
                        maintainer: None,
                        dependencies: Vec::new(),
                        install_date: None,
                        enrichment: None,
                    });
                    if packages.len() >= 50 {
                        break;
                    }
                    i += 2;
                    continue;
                }
            }
            i += 1;
        }

        Ok(packages)
    }
}
