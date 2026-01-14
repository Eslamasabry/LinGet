use crate::backends::PackageBackend;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus};

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
        let output = std::process::Command::new("zypper")
            .args(["search", "-i", "--no-type-display"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines().skip(4) {
            let parts: Vec<&str> = line.split(" | ").collect();
            if parts.len() >= 3 {
                let name = parts[1].trim().to_string();
                if !name.is_empty() {
                    packages.push(Package {
                        name,
                        version: parts[2].trim().to_string(),
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
                        update_category: None,
                    });
                }
            }
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }

    async fn install(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pkexec")
            .args(["zypper", "install", "-y", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("zypper install failed");
        }

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pkexec")
            .args(["zypper", "remove", "-y", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("zypper remove failed");
        }

        Ok(())
    }

    async fn update(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pkexec")
            .args(["zypper", "update", "-y", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("zypper update failed");
        }

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = std::process::Command::new("zypper")
            .args(["search", query])
            .output()?;

        Ok(Vec::new())
    }

    fn source(&self) -> PackageSource {
        PackageSource::Zypper
    }
}
