use crate::backends::PackageBackend;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus};

pub struct PacmanBackend;

impl PacmanBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PacmanBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for PacmanBackend {
    fn is_available() -> bool {
        which::which("pacman").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = std::process::Command::new("pacman")
            .args(["-Q", "--noheader"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: parts.get(1).unwrap_or(&"").to_string(),
                    available_version: None,
                    description: String::new(),
                    source: PackageSource::Pacman,
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
        Ok(Vec::new())
    }

    async fn install(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pkexec")
            .args(["pacman", "-S", "--noconfirm", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("pacman install failed");
        }

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pkexec")
            .args(["pacman", "-R", "--noconfirm", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("pacman remove failed");
        }

        Ok(())
    }

    async fn update(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pkexec")
            .args(["pacman", "-S", "--noconfirm", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("pacman update failed");
        }

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = std::process::Command::new("pacman")
            .args(["-Ss", query])
            .output()?;

        Ok(Vec::new())
    }

    fn source(&self) -> PackageSource {
        PackageSource::Pacman
    }
}
