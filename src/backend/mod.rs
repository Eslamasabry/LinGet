mod traits;
mod apt;
mod flatpak;
mod snap;
mod npm;
mod pip;
mod deb;
mod appimage;

pub use traits::*;
pub use apt::AptBackend;
pub use flatpak::FlatpakBackend;
pub use snap::SnapBackend;
pub use npm::NpmBackend;
pub use pip::PipBackend;
pub use deb::DebBackend;
pub use appimage::AppImageBackend;

use crate::models::{Package, PackageSource};
use anyhow::Result;
use std::collections::HashMap;

/// Manager that coordinates all package backends
pub struct PackageManager {
    backends: HashMap<PackageSource, Box<dyn PackageBackend>>,
}

impl PackageManager {
    pub fn new() -> Self {
        let mut backends: HashMap<PackageSource, Box<dyn PackageBackend>> = HashMap::new();

        // Add available backends
        if AptBackend::is_available() {
            backends.insert(PackageSource::Apt, Box::new(AptBackend::new()));
        }
        if FlatpakBackend::is_available() {
            backends.insert(PackageSource::Flatpak, Box::new(FlatpakBackend::new()));
        }
        if SnapBackend::is_available() {
            backends.insert(PackageSource::Snap, Box::new(SnapBackend::new()));
        }
        if NpmBackend::is_available() {
            backends.insert(PackageSource::Npm, Box::new(NpmBackend::new()));
        }
        if PipBackend::is_available() {
            backends.insert(PackageSource::Pip, Box::new(PipBackend::new()));
        }
        if DebBackend::is_available() {
            backends.insert(PackageSource::Deb, Box::new(DebBackend::new()));
        }
        if AppImageBackend::is_available() {
            backends.insert(PackageSource::AppImage, Box::new(AppImageBackend::new()));
        }

        Self { backends }
    }

    pub async fn list_all_installed(&self) -> Result<Vec<Package>> {
        use futures::future::join_all;

        // Load all backends in parallel
        let futures: Vec<_> = self.backends.values()
            .map(|backend| backend.list_installed())
            .collect();

        let results = join_all(futures).await;

        let mut all_packages = Vec::new();
        for result in results {
            match result {
                Ok(packages) => all_packages.extend(packages),
                Err(e) => tracing::warn!("Failed to list packages: {}", e),
            }
        }

        all_packages.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(all_packages)
    }

    pub async fn check_all_updates(&self) -> Result<Vec<Package>> {
        use futures::future::join_all;

        // Check all backends in parallel
        let futures: Vec<_> = self.backends.values()
            .map(|backend| backend.check_updates())
            .collect();

        let results = join_all(futures).await;

        let mut all_updates = Vec::new();
        for result in results {
            match result {
                Ok(packages) => all_updates.extend(packages),
                Err(e) => tracing::warn!("Failed to check updates: {}", e),
            }
        }

        Ok(all_updates)
    }

    pub async fn install(&self, package: &Package) -> Result<()> {
        if let Some(backend) = self.backends.get(&package.source) {
            backend.install(&package.name).await
        } else {
            anyhow::bail!("No backend available for {:?}", package.source)
        }
    }

    pub async fn remove(&self, package: &Package) -> Result<()> {
        if let Some(backend) = self.backends.get(&package.source) {
            backend.remove(&package.name).await
        } else {
            anyhow::bail!("No backend available for {:?}", package.source)
        }
    }

    pub async fn update(&self, package: &Package) -> Result<()> {
        if let Some(backend) = self.backends.get(&package.source) {
            backend.update(&package.name).await
        } else {
            anyhow::bail!("No backend available for {:?}", package.source)
        }
    }
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::new()
    }
}
