use crate::backends::PackageBackend;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus};

pub struct PipxBackend;

impl PipxBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PipxBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for PipxBackend {
    fn is_available() -> bool {
        which::which("pipx").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = std::process::Command::new("pipx").args(["list"]).output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            if line.contains("package") {
                let parts: Vec<&str> = line.split("  ").collect();
                if parts.len() >= 2 {
                    let name = parts[0].trim();
                    let version = parts.get(1).unwrap_or(&"").trim().to_string();

                    packages.push(Package {
                        name: name.to_string(),
                        version,
                        available_version: None,
                        description: String::new(),
                        source: PackageSource::Pipx,
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
        let output = std::process::Command::new("pipx")
            .args(["install", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("pipx install failed");
        }

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pipx")
            .args(["uninstall", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("pipx uninstall failed");
        }

        Ok(())
    }

    async fn update(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("pipx")
            .args(["upgrade", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("pipx upgrade failed");
        }

        Ok(())
    }

    async fn search(&self, _query: &str) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }

    fn source(&self) -> PackageSource {
        PackageSource::Pipx
    }
}
