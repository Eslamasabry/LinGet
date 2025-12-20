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
pub use pkexec::{run_command, run_pkexec, Suggest, SUGGEST_PREFIX};
pub use providers::{detect_available_providers, detect_provider, detect_providers, ProviderStatus};
pub use snap::SnapBackend;
pub use traits::*;
pub use zypper::ZypperBackend;

use crate::models::{FlatpakMetadata, FlatpakPermission, Package, PackageSource, Repository};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use tracing::{debug, error, info, instrument, warn};

/// Manager that coordinates all package backends
pub struct PackageManager {
    backends: HashMap<PackageSource, Box<dyn PackageBackend>>,
    enabled_sources: HashSet<PackageSource>,
}

impl PackageManager {
    pub fn new() -> Self {
        info!("Initializing PackageManager, detecting available backends");
        let mut backends: HashMap<PackageSource, Box<dyn PackageBackend>> = HashMap::new();

        // Add available backends with logging
        let mut check_backend = |source: PackageSource, available: bool, backend: Box<dyn PackageBackend>| {
            if available {
                debug!(source = ?source, "Backend available");
                backends.insert(source, backend);
            } else {
                debug!(source = ?source, "Backend not available");
            }
        };

        check_backend(PackageSource::Apt, AptBackend::is_available(), Box::new(AptBackend::new()));
        check_backend(PackageSource::Dnf, DnfBackend::is_available(), Box::new(DnfBackend::new()));
        check_backend(PackageSource::Pacman, PacmanBackend::is_available(), Box::new(PacmanBackend::new()));
        check_backend(PackageSource::Zypper, ZypperBackend::is_available(), Box::new(ZypperBackend::new()));
        check_backend(PackageSource::Flatpak, FlatpakBackend::is_available(), Box::new(FlatpakBackend::new()));
        check_backend(PackageSource::Snap, SnapBackend::is_available(), Box::new(SnapBackend::new()));
        check_backend(PackageSource::Npm, NpmBackend::is_available(), Box::new(NpmBackend::new()));
        check_backend(PackageSource::Pip, PipBackend::is_available(), Box::new(PipBackend::new()));
        check_backend(PackageSource::Pipx, PipxBackend::is_available(), Box::new(PipxBackend::new()));
        check_backend(PackageSource::Cargo, CargoBackend::is_available(), Box::new(CargoBackend::new()));
        check_backend(PackageSource::Brew, BrewBackend::is_available(), Box::new(BrewBackend::new()));
        check_backend(PackageSource::Aur, AurBackend::is_available(), Box::new(AurBackend::new()));
        check_backend(PackageSource::Conda, CondaBackend::is_available(), Box::new(CondaBackend::new()));
        check_backend(PackageSource::Mamba, MambaBackend::is_available(), Box::new(MambaBackend::new()));
        check_backend(PackageSource::Dart, DartBackend::is_available(), Box::new(DartBackend::new()));
        check_backend(PackageSource::Deb, DebBackend::is_available(), Box::new(DebBackend::new()));
        check_backend(PackageSource::AppImage, AppImageBackend::is_available(), Box::new(AppImageBackend::new()));

        let enabled_sources = backends.keys().copied().collect();
        info!(
            available_backends = backends.len(),
            backends = ?backends.keys().collect::<Vec<_>>(),
            "PackageManager initialized"
        );

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
        debug!(
            enabled_sources = ?self.enabled_sources,
            "Updated enabled sources"
        );
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

    #[instrument(skip(self), level = "debug")]
    pub async fn list_all_installed(&self) -> Result<Vec<Package>> {
        use futures::future::join_all;

        let enabled_count = self.enabled_sources.len();
        debug!(enabled_backends = enabled_count, "Listing installed packages from all enabled backends");

        // Load all backends in parallel
        let futures: Vec<_> = self
            .enabled_backends()
            .map(|(source, backend)| {
                let source = *source;
                async move { (source, backend.list_installed().await) }
            })
            .collect();

        let results = join_all(futures).await;

        let mut all_packages = Vec::new();
        let mut success_count = 0;
        let mut error_count = 0;

        for (source, result) in results {
            match result {
                Ok(packages) => {
                    debug!(source = ?source, package_count = packages.len(), "Listed packages from backend");
                    success_count += 1;
                    all_packages.extend(packages);
                }
                Err(e) => {
                    error_count += 1;
                    warn!(source = ?source, error = %e, "Failed to list packages from backend");
                }
            }
        }

        all_packages.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        info!(
            total_packages = all_packages.len(),
            successful_backends = success_count,
            failed_backends = error_count,
            "Finished listing installed packages"
        );

        Ok(all_packages)
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn check_all_updates(&self) -> Result<Vec<Package>> {
        use futures::future::join_all;

        debug!("Checking for updates from all enabled backends");

        // Check all backends in parallel
        let futures: Vec<_> = self
            .enabled_backends()
            .map(|(source, backend)| {
                let source = *source;
                async move { (source, backend.check_updates().await) }
            })
            .collect();

        let results = join_all(futures).await;

        let mut all_updates = Vec::new();
        let mut success_count = 0;
        let mut error_count = 0;

        for (source, result) in results {
            match result {
                Ok(packages) => {
                    if !packages.is_empty() {
                        debug!(source = ?source, update_count = packages.len(), "Found updates");
                    }
                    success_count += 1;
                    all_updates.extend(packages);
                }
                Err(e) => {
                    error_count += 1;
                    warn!(source = ?source, error = %e, "Failed to check updates from backend");
                }
            }
        }

        info!(
            total_updates = all_updates.len(),
            successful_backends = success_count,
            failed_backends = error_count,
            "Finished checking for updates"
        );

        Ok(all_updates)
    }

    #[instrument(skip(self), fields(package = %package.name, source = ?package.source))]
    pub async fn install(&self, package: &Package) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        if !self.enabled_sources.contains(&package.source) {
            warn!(source = ?package.source, "Attempted to install from disabled source");
            anyhow::bail!("{} source is disabled. Enable it in settings to install packages from this source.", package.source);
        }

        if let Some(backend) = self.backends.get(&package.source) {
            info!(package = %package.name, source = ?package.source, "Installing package");
            match backend.install(&package.name).await {
                Ok(()) => {
                    info!(package = %package.name, source = ?package.source, "Package installed successfully");
                    Ok(())
                }
                Err(e) => {
                    error!(package = %package.name, source = ?package.source, error = %e, "Failed to install package");
                    Err(e)
                }
            }
        } else {
            error!(source = ?package.source, "No backend available for source");
            anyhow::bail!("No backend available for {}. This package source may not be installed on your system.", package.source)
        }
    }

    #[instrument(skip(self), fields(package = %package.name, source = ?package.source))]
    pub async fn remove(&self, package: &Package) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        if !self.enabled_sources.contains(&package.source) {
            warn!(source = ?package.source, "Attempted to remove from disabled source");
            anyhow::bail!("{} source is disabled. Enable it in settings to manage packages from this source.", package.source);
        }

        if let Some(backend) = self.backends.get(&package.source) {
            info!(package = %package.name, source = ?package.source, "Removing package");
            match backend.remove(&package.name).await {
                Ok(()) => {
                    info!(package = %package.name, source = ?package.source, "Package removed successfully");
                    Ok(())
                }
                Err(e) => {
                    error!(package = %package.name, source = ?package.source, error = %e, "Failed to remove package");
                    Err(e)
                }
            }
        } else {
            error!(source = ?package.source, "No backend available for source");
            anyhow::bail!("No backend available for {}. This package source may not be installed on your system.", package.source)
        }
    }

    #[instrument(skip(self), fields(package = %package.name, source = ?package.source))]
    pub async fn update(&self, package: &Package) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        if !self.enabled_sources.contains(&package.source) {
            warn!(source = ?package.source, "Attempted to update from disabled source");
            anyhow::bail!("{} source is disabled. Enable it in settings to manage packages from this source.", package.source);
        }

        if let Some(backend) = self.backends.get(&package.source) {
            info!(package = %package.name, source = ?package.source, "Updating package");
            match backend.update(&package.name).await {
                Ok(()) => {
                    info!(package = %package.name, source = ?package.source, "Package updated successfully");
                    Ok(())
                }
                Err(e) => {
                    error!(package = %package.name, source = ?package.source, error = %e, "Failed to update package");
                    Err(e)
                }
            }
        } else {
            error!(source = ?package.source, "No backend available for source");
            anyhow::bail!("No backend available for {}. This package source may not be installed on your system.", package.source)
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

    #[instrument(skip(self), fields(query = %query))]
    pub async fn search(&self, query: &str) -> Result<Vec<Package>> {
        use futures::future::join_all;

        debug!(query = %query, "Searching across all enabled backends");

        let futures: Vec<_> = self
            .enabled_backends()
            .map(|(source, backend)| {
                let source = *source;
                async move { (source, backend.search(query).await) }
            })
            .collect();

        let results = join_all(futures).await;

        let mut all_results = Vec::new();
        let mut success_count = 0;
        let mut error_count = 0;

        for (source, result) in results {
            match result {
                Ok(packages) => {
                    if !packages.is_empty() {
                        debug!(source = ?source, result_count = packages.len(), "Search results from backend");
                    }
                    success_count += 1;
                    all_results.extend(packages);
                }
                Err(e) => {
                    error_count += 1;
                    warn!(source = ?source, error = %e, "Search failed for backend");
                }
            }
        }

        all_results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        info!(
            query = %query,
            total_results = all_results.len(),
            successful_backends = success_count,
            failed_backends = error_count,
            "Search completed"
        );

        Ok(all_results)
    }

    // =========================================================================
    // Flatpak-specific methods for sandbox management
    // =========================================================================

    /// Get detailed Flatpak metadata including sandbox permissions for an application
    pub async fn get_flatpak_metadata(&self, app_id: &str) -> Result<FlatpakMetadata> {
        if !self.backends.contains_key(&PackageSource::Flatpak) {
            anyhow::bail!("Flatpak backend is not available");
        }

        let backend = FlatpakBackend::new();
        backend.get_metadata(app_id).await
    }

    /// Get the permission overrides for a Flatpak application
    pub async fn get_flatpak_overrides(&self, app_id: &str) -> Result<Vec<FlatpakPermission>> {
        if !self.backends.contains_key(&PackageSource::Flatpak) {
            anyhow::bail!("Flatpak backend is not available");
        }

        let backend = FlatpakBackend::new();
        backend.get_overrides(app_id).await
    }

    /// Add a permission override for a Flatpak application
    pub async fn add_flatpak_override(&self, app_id: &str, permission: &str) -> Result<()> {
        if !self.backends.contains_key(&PackageSource::Flatpak) {
            anyhow::bail!("Flatpak backend is not available");
        }

        let backend = FlatpakBackend::new();
        backend.add_override(app_id, permission).await
    }

    /// Remove a permission override for a Flatpak application
    pub async fn remove_flatpak_override(&self, app_id: &str, permission: &str) -> Result<()> {
        if !self.backends.contains_key(&PackageSource::Flatpak) {
            anyhow::bail!("Flatpak backend is not available");
        }

        let backend = FlatpakBackend::new();
        backend.remove_override(app_id, permission).await
    }

    /// Reset all overrides for a Flatpak application
    pub async fn reset_flatpak_overrides(&self, app_id: &str) -> Result<()> {
        if !self.backends.contains_key(&PackageSource::Flatpak) {
            anyhow::bail!("Flatpak backend is not available");
        }

        let backend = FlatpakBackend::new();
        backend.reset_overrides(app_id).await
    }

    /// List all Flatpak runtimes installed on the system
    pub async fn list_flatpak_runtimes(&self) -> Result<Vec<Package>> {
        if !self.backends.contains_key(&PackageSource::Flatpak) {
            anyhow::bail!("Flatpak backend is not available");
        }

        let backend = FlatpakBackend::new();
        backend.list_runtimes().await
    }

    /// Check if a Flatpak application is well sandboxed
    pub async fn is_flatpak_well_sandboxed(&self, app_id: &str) -> Result<bool> {
        if !self.backends.contains_key(&PackageSource::Flatpak) {
            anyhow::bail!("Flatpak backend is not available");
        }

        let backend = FlatpakBackend::new();
        backend.is_well_sandboxed(app_id).await
    }
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::new()
    }
}
