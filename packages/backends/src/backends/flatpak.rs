use crate::backends::PackageBackend;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus};
use std::path::PathBuf;
use which::which;

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
        which("flatpak").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = std::process::Command::new("flatpak")
            .args([
                "list",
                "--columns=application,name,version,description",
                "--app",
            ])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.splitn(4, '\t').collect();
            if parts.len() >= 2 {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: parts.get(2).unwrap_or(&"").to_string(),
                    available_version: None,
                    description: parts.get(3).unwrap_or(&"").to_string(),
                    source: PackageSource::Flatpak,
                    status: PackageStatus::Installed,
                    size: None,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                    update_category: None,
                });
            }
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        let output = std::process::Command::new("flatpak")
            .args(["update", "--dry-run"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            if line.starts_with("Info:") || line.starts_with("Updating") {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: String::new(),
                    available_version: Some(parts.get(1).unwrap_or(&"").to_string()),
                    description: String::new(),
                    source: PackageSource::Flatpak,
                    status: PackageStatus::UpdateAvailable,
                    size: None,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                    update_category: None,
                });
            }
        }

        Ok(packages)
    }

    async fn install(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pkexec")
            .args(["flatpak", "install", "-y", name])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to install flatpak: {}", stderr);
        }

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pkexec")
            .args(["flatpak", "uninstall", "-y", name])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to uninstall flatpak: {}", stderr);
        }

        Ok(())
    }

    async fn update(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pkexec")
            .args(["flatpak", "update", "-y", name])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to update flatpak: {}", stderr);
        }

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = std::process::Command::new("flatpak")
            .args([
                "search",
                "--columns=application,name,version,description",
                query,
            ])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(4, '\t').collect();
            if parts.len() >= 2 {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: parts.get(2).unwrap_or(&"").to_string(),
                    available_version: None,
                    description: parts.get(3).unwrap_or(&"").to_string(),
                    source: PackageSource::Flatpak,
                    status: PackageStatus::NotInstalled,
                    size: None,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                    update_category: None,
                });
            }
        }

        Ok(packages)
    }

    fn source(&self) -> PackageSource {
        PackageSource::Flatpak
    }
}
