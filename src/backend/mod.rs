mod appimage;
mod apt;
mod aur;
mod brew;
mod cargo;
mod conda;
mod dart;
mod deb;
mod dnf;
mod flatpak;
mod mamba;
mod npm;
mod pacman;
mod pip;
mod pipx;
mod pkexec;
mod providers;
mod snap;
mod traits;
mod zypper;

pub use appimage::AppImageBackend;
pub use apt::AptBackend;
pub use aur::AurBackend;
pub use brew::BrewBackend;
pub use cargo::CargoBackend;
pub use conda::CondaBackend;
pub use dart::DartBackend;
pub use deb::DebBackend;
pub use dnf::DnfBackend;
pub use flatpak::FlatpakBackend;
pub use mamba::MambaBackend;
pub use npm::NpmBackend;
pub use pacman::PacmanBackend;
pub use pip::PipBackend;
pub use pipx::PipxBackend;
pub use pkexec::{run_pkexec, Suggest, SUGGEST_PREFIX};
pub use providers::{detect_available_providers, detect_provider, detect_providers, ProviderStatus};
pub use snap::SnapBackend;
pub use traits::*;
pub use zypper::ZypperBackend;

use crate::models::{FlatpakMetadata, FlatpakPermission, Package, PackageSource, Repository};
use anyhow::Result;
use std::collections::{HashMap, HashSet};

/// Manager that coordinates all package backends
pub struct PackageManager {
    backends: HashMap<PackageSource, Box<dyn PackageBackend>>,
    enabled_sources: HashSet<PackageSource>,
}

impl PackageManager {
    pub fn new() -> Self {
        let mut backends: HashMap<PackageSource, Box<dyn PackageBackend>> = HashMap::new();

        // Add available backends
        if AptBackend::is_available() {
            backends.insert(PackageSource::Apt, Box::new(AptBackend::new()));
        }
        if DnfBackend::is_available() {
            backends.insert(PackageSource::Dnf, Box::new(DnfBackend::new()));
        }
        if PacmanBackend::is_available() {
            backends.insert(PackageSource::Pacman, Box::new(PacmanBackend::new()));
        }
        if ZypperBackend::is_available() {
            backends.insert(PackageSource::Zypper, Box::new(ZypperBackend::new()));
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
        if PipxBackend::is_available() {
            backends.insert(PackageSource::Pipx, Box::new(PipxBackend::new()));
        }
        if CargoBackend::is_available() {
            backends.insert(PackageSource::Cargo, Box::new(CargoBackend::new()));
        }
        if BrewBackend::is_available() {
            backends.insert(PackageSource::Brew, Box::new(BrewBackend::new()));
        }
        if AurBackend::is_available() {
            backends.insert(PackageSource::Aur, Box::new(AurBackend::new()));
        }
        if CondaBackend::is_available() {
            backends.insert(PackageSource::Conda, Box::new(CondaBackend::new()));
        }
        if MambaBackend::is_available() {
            backends.insert(PackageSource::Mamba, Box::new(MambaBackend::new()));
        }
        if DartBackend::is_available() {
            backends.insert(PackageSource::Dart, Box::new(DartBackend::new()));
        }
        if DebBackend::is_available() {
            backends.insert(PackageSource::Deb, Box::new(DebBackend::new()));
        }
        if AppImageBackend::is_available() {
            backends.insert(PackageSource::AppImage, Box::new(AppImageBackend::new()));
        }

        let enabled_sources = backends.keys().copied().collect();
        Self {
            backends,
            enabled_sources,
        }
    }

    pub fn set_enabled_sources(&mut self, enabled_sources: HashSet<PackageSource>) {
        // Only enable sources that have an available backend.
        self.enabled_sources = enabled_sources
            .into_iter()
            .filter(|s| self.backends.contains_key(s))
            .collect();
    }

    pub fn available_sources(&self) -> HashSet<PackageSource> {
        self.backends.keys().copied().collect()
    }

    fn validate_package_name(name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            anyhow::bail!("Package name is empty");
        }
        if name.starts_with('-') {
            anyhow::bail!("Invalid package name '{}'", name);
        }
        if name.len() > 256 {
            anyhow::bail!("Package name is too long");
        }
        if name.chars().any(|c| c == '\0' || c.is_control()) {
            anyhow::bail!("Invalid package name '{}'", name);
        }
        Ok(())
    }

    fn enabled_backends(&self) -> impl Iterator<Item = (&PackageSource, &Box<dyn PackageBackend>)> {
        self.backends
            .iter()
            .filter(|(source, _)| self.enabled_sources.contains(source))
    }

    pub async fn list_all_installed(&self) -> Result<Vec<Package>> {
        use futures::future::join_all;

        // Load all backends in parallel
        let futures: Vec<_> = self
            .enabled_backends()
            .map(|(_, backend)| backend.list_installed())
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
        let futures: Vec<_> = self
            .enabled_backends()
            .map(|(_, backend)| backend.check_updates())
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
        Self::validate_package_name(&package.name)?;
        if !self.enabled_sources.contains(&package.source) {
            anyhow::bail!("{:?} source is disabled", package.source);
        }

        if let Some(backend) = self.backends.get(&package.source) {
            backend.install(&package.name).await
        } else {
            anyhow::bail!("No backend available for {:?}", package.source)
        }
    }

    pub async fn remove(&self, package: &Package) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        if !self.enabled_sources.contains(&package.source) {
            anyhow::bail!("{:?} source is disabled", package.source);
        }

        if let Some(backend) = self.backends.get(&package.source) {
            backend.remove(&package.name).await
        } else {
            anyhow::bail!("No backend available for {:?}", package.source)
        }
    }

    pub async fn update(&self, package: &Package) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        if !self.enabled_sources.contains(&package.source) {
            anyhow::bail!("{:?} source is disabled", package.source);
        }

        if let Some(backend) = self.backends.get(&package.source) {
            backend.update(&package.name).await
        } else {
            anyhow::bail!("No backend available for {:?}", package.source)
        }
    }

    pub async fn downgrade(&self, package: &Package) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        if !self.enabled_sources.contains(&package.source) {
            anyhow::bail!("{:?} source is disabled", package.source);
        }

        if let Some(backend) = self.backends.get(&package.source) {
            backend.downgrade(&package.name).await
        } else {
            anyhow::bail!("No backend available for {:?}", package.source)
        }
    }

    pub async fn downgrade_to(&self, package: &Package, version: &str) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        if !self.enabled_sources.contains(&package.source) {
            anyhow::bail!("{:?} source is disabled", package.source);
        }

        if let Some(backend) = self.backends.get(&package.source) {
            backend.downgrade_to(&package.name, version).await
        } else {
            anyhow::bail!("No backend available for {:?}", package.source)
        }
    }

    #[allow(dead_code)]
    pub async fn available_downgrade_versions(&self, package: &Package) -> Result<Vec<String>> {
        Self::validate_package_name(&package.name)?;
        if !self.enabled_sources.contains(&package.source) {
            anyhow::bail!("{:?} source is disabled", package.source);
        }

        if let Some(backend) = self.backends.get(&package.source) {
            backend.available_downgrade_versions(&package.name).await
        } else {
            anyhow::bail!("No backend available for {:?}", package.source)
        }
    }

    pub async fn get_changelog(&self, package: &Package) -> Result<Option<String>> {
        Self::validate_package_name(&package.name)?;

        if let Some(backend) = self.backends.get(&package.source) {
            backend.get_changelog(&package.name).await
        } else {
            Ok(None)
        }
    }

    pub async fn list_repositories(&self, source: PackageSource) -> Result<Vec<Repository>> {
        if let Some(backend) = self.backends.get(&source) {
            backend.list_repositories().await
        } else {
            Ok(Vec::new())
        }
    }

    #[allow(dead_code)] // Useful for future multi-backend repository listing
    pub async fn list_all_repositories(&self) -> Result<Vec<Repository>> {
        use futures::future::join_all;

        let futures: Vec<_> = self
            .enabled_backends()
            .map(|(_, backend)| backend.list_repositories())
            .collect();

        let results = join_all(futures).await;

        let mut all_repos = Vec::new();
        for result in results {
            match result {
                Ok(repos) => all_repos.extend(repos),
                Err(e) => tracing::warn!("Failed to list repositories: {}", e),
            }
        }

        Ok(all_repos)
    }

    pub async fn add_repository(
        &self,
        source: PackageSource,
        url: &str,
        name: Option<&str>,
    ) -> Result<()> {
        if let Some(backend) = self.backends.get(&source) {
            backend.add_repository(url, name).await
        } else {
            anyhow::bail!("No backend available for {:?}", source)
        }
    }

    pub async fn remove_repository(&self, source: PackageSource, name: &str) -> Result<()> {
        if let Some(backend) = self.backends.get(&source) {
            backend.remove_repository(name).await
        } else {
            anyhow::bail!("No backend available for {:?}", source)
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Package>> {
        use futures::future::join_all;

        let futures: Vec<_> = self
            .enabled_backends()
            .map(|(_, backend)| backend.search(query))
            .collect();

        let results = join_all(futures).await;

        let mut all_results = Vec::new();
        for result in results {
            match result {
                Ok(packages) => all_results.extend(packages),
                Err(e) => tracing::warn!("Search failed: {}", e),
            }
        }

        all_results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(all_results)
    }
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::new()
    }
}
