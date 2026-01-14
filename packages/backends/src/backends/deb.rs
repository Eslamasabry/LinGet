use crate::backends::PackageBackend;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus};

pub struct DebBackend;

impl DebBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DebBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for DebBackend {
    fn is_available() -> bool {
        which::which("dpkg").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = std::process::Command::new("dpkg")
            .args(["-l", "--no-pager"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            if line.starts_with("ii ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    packages.push(Package {
                        name: parts[1].to_string(),
                        version: parts[2].to_string(),
                        available_version: None,
                        description: String::new(),
                        source: PackageSource::Deb,
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

    async fn install(&self, path: &str) -> Result<()> {
        let output = std::process::Command::new("pkexec")
            .args(["dpkg", "-i", path])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("dpkg install failed");
        }

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pkexec")
            .args(["dpkg", "-r", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("dpkg remove failed");
        }

        Ok(())
    }

    async fn update(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn search(&self, _query: &str) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }

    fn source(&self) -> PackageSource {
        PackageSource::Deb
    }
}
