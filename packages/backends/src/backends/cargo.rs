use crate::backends::PackageBackend;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus};

pub struct CargoBackend;

impl CargoBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CargoBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for CargoBackend {
    fn is_available() -> bool {
        which::which("cargo").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = std::process::Command::new("cargo")
            .args(["install", "--list"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            if line.contains(" v") {
                let parts: Vec<&str> = line.split(" v").collect();
                let name = parts[0].trim();
                let version = parts.get(1).unwrap_or(&"").trim().to_string();

                packages.push(Package {
                    name: name.to_string(),
                    version,
                    available_version: None,
                    description: String::new(),
                    source: PackageSource::Cargo,
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
        let output = std::process::Command::new("cargo")
            .args(["install", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("cargo install failed");
        }

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("cargo")
            .args(["uninstall", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("cargo uninstall failed");
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
        PackageSource::Cargo
    }
}
