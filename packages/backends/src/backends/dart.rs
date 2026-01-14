use crate::backends::PackageBackend;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus};

pub struct DartBackend;

impl DartBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DartBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for DartBackend {
    fn is_available() -> bool {
        which::which("dart").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = std::process::Command::new("dart")
            .args(["pub", "global", "list"])
            .output()?;

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
                    source: PackageSource::Dart,
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
        let output = std::process::Command::new("dart")
            .args(["pub", "global", "activate", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("dart pub global activate failed");
        }

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("dart")
            .args(["pub", "global", "deactivate", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("dart pub global deactivate failed");
        }

        Ok(())
    }

    async fn update(&self, name: &str) -> Result<()> {
        let output = std::process::Command::new("dart")
            .args(["pub", "global", "activate", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("dart update failed");
        }

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = std::process::Command::new("dart")
            .args(["pub", "global", "list"])
            .output()?;

        Ok(Vec::new())
    }

    fn source(&self) -> PackageSource {
        PackageSource::Dart
    }
}
