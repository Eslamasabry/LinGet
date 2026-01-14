use crate::backends::PackageBackend;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus};

pub struct AurBackend;

impl AurBackend {
    pub fn new() -> Self {
        Self
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
        Ok(Vec::new())
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }

    async fn install(&self, name: &str) -> Result<()> {
        let aur_helper = if which::which("yay").is_ok() {
            "yay"
        } else {
            "paru"
        };
        let output = std::process::Command::new("pkexec")
            .args([aur_helper, "-S", "--noconfirm", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("AUR install failed");
        }

        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let aur_helper = if which::which("yay").is_ok() {
            "yay"
        } else {
            "paru"
        };
        let output = std::process::Command::new("pkexec")
            .args([aur_helper, "-R", "--noconfirm", name])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("AUR remove failed");
        }

        Ok(())
    }

    async fn update(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let aur_helper = if which::which("yay").is_ok() {
            "yay"
        } else {
            "paru"
        };
        let output = std::process::Command::new(aur_helper)
            .args(["-Ss", query])
            .output()?;

        Ok(Vec::new())
    }

    fn source(&self) -> PackageSource {
        PackageSource::Aur
    }
}
