use crate::streaming::StreamLine;
use crate::{run_pkexec, run_pkexec_with_logs, PackageBackend, Suggest};
use anyhow::{Context, Result};
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus, Repository};
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::mpsc;
use which::which;

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
mod snap;
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
pub use snap::SnapBackend;
pub use zypper::ZypperBackend;

use crate::LockStatus;

pub struct PackageManager {
    backends: std::collections::HashMap<PackageSource, Box<dyn PackageBackend>>,
    enabled_sources: std::collections::HashSet<PackageSource>,
}

impl PackageManager {
    pub fn new() -> Self {
        tracing::info!("Initializing PackageManager, detecting available backends");
        let mut backends: std::collections::HashMap<PackageSource, Box<dyn PackageBackend>> =
            std::collections::HashMap::new();

        let mut check_backend =
            |source: PackageSource, available: bool, backend: Box<dyn PackageBackend>| {
                if available {
                    tracing::debug!(source = ?source, "Backend available");
                    backends.insert(source, backend);
                } else {
                    tracing::debug!(source = ?source, "Backend not available");
                }
            };

        check_backend(
            PackageSource::Apt,
            AptBackend::is_available(),
            Box::<AptBackend>::default(),
        );
        check_backend(
            PackageSource::Dnf,
            DnfBackend::is_available(),
            Box::<DnfBackend>::default(),
        );
        check_backend(
            PackageSource::Pacman,
            PacmanBackend::is_available(),
            Box::<PacmanBackend>::default(),
        );
        check_backend(
            PackageSource::Zypper,
            ZypperBackend::is_available(),
            Box::<ZypperBackend>::default(),
        );
        check_backend(
            PackageSource::Flatpak,
            FlatpakBackend::is_available(),
            Box::<FlatpakBackend>::default(),
        );
        check_backend(
            PackageSource::Snap,
            SnapBackend::is_available(),
            Box::<SnapBackend>::default(),
        );
        check_backend(
            PackageSource::Npm,
            NpmBackend::is_available(),
            Box::<NpmBackend>::default(),
        );
        check_backend(
            PackageSource::Pip,
            PipBackend::is_available(),
            Box::<PipBackend>::default(),
        );
        check_backend(
            PackageSource::Pipx,
            PipxBackend::is_available(),
            Box::<PipxBackend>::default(),
        );
        check_backend(
            PackageSource::Cargo,
            CargoBackend::is_available(),
            Box::<CargoBackend>::default(),
        );
        check_backend(
            PackageSource::Brew,
            BrewBackend::is_available(),
            Box::<BrewBackend>::default(),
        );
        check_backend(
            PackageSource::Aur,
            AurBackend::is_available(),
            Box::<AurBackend>::default(),
        );
        check_backend(
            PackageSource::Conda,
            CondaBackend::is_available(),
            Box::<CondaBackend>::default(),
        );
        check_backend(
            PackageSource::Mamba,
            MambaBackend::is_available(),
            Box::<MambaBackend>::default(),
        );
        check_backend(
            PackageSource::Dart,
            DartBackend::is_available(),
            Box::<DartBackend>::default(),
        );
        check_backend(
            PackageSource::Deb,
            DebBackend::is_available(),
            Box::<DebBackend>::default(),
        );
        check_backend(
            PackageSource::AppImage,
            AppImageBackend::is_available(),
            Box::<AppImageBackend>::default(),
        );

        let enabled_sources = backends.keys().copied().collect();
        tracing::info!(
            available_backends = backends.len(),
            backends = ?backends.keys().collect::<Vec<_>>(),
            "PackageManager initialized"
        );

        Self {
            backends,
            enabled_sources,
        }
    }

    pub fn set_enabled_sources(
        &mut self,
        enabled_sources: std::collections::HashSet<PackageSource>,
    ) {
        self.enabled_sources = enabled_sources
            .into_iter()
            .filter(|s| self.backends.contains_key(s))
            .collect();
        tracing::debug!(
            enabled_sources = ?self.enabled_sources,
            "Updated enabled sources"
        );
    }

    pub fn available_sources(&self) -> std::collections::HashSet<PackageSource> {
        self.backends.keys().copied().collect()
    }

    pub fn get_backend(&self, source: PackageSource) -> Option<&dyn PackageBackend> {
        self.backends.get(&source).map(|b| b.as_ref())
    }

    fn enabled_backends(&self) -> impl Iterator<Item = (&PackageSource, &Box<dyn PackageBackend>)> {
        self.backends
            .iter()
            .filter(|(source, _)| self.enabled_sources.contains(source))
    }

    pub async fn list_all_installed(&self) -> Result<Vec<Package>> {
        use futures::future::join_all;

        let enabled_count = self.enabled_sources.len();
        tracing::debug!(
            enabled_backends = enabled_count,
            "Listing installed packages from all enabled backends"
        );

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
                    tracing::debug!(
                        source = ?source,
                        package_count = packages.len(),
                        "Listed packages from backend"
                    );
                    success_count += 1;
                    all_packages.extend(packages);
                }
                Err(e) => {
                    error_count += 1;
                    tracing::warn!(source = ?source, error = %e, "Failed to list packages from backend");
                }
            }
        }

        all_packages.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        tracing::info!(
            total_packages = all_packages.len(),
            successful_backends = success_count,
            failed_backends = error_count,
            "Finished listing installed packages"
        );

        Ok(all_packages)
    }

    pub async fn check_all_updates(&self) -> Result<Vec<Package>> {
        use futures::future::join_all;

        tracing::debug!("Checking for updates from all enabled backends");

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
                        tracing::debug!(
                            source = ?source,
                            update_count = packages.len(),
                            "Found updates"
                        );
                    }
                    success_count += 1;
                    all_updates.extend(packages);
                }
                Err(e) => {
                    error_count += 1;
                    tracing::warn!(source = ?source, error = %e, "Failed to check updates from backend");
                }
            }
        }

        tracing::info!(
            total_updates = all_updates.len(),
            successful_backends = success_count,
            failed_backends = error_count,
            "Finished checking for updates"
        );

        Ok(all_updates)
    }

    pub async fn install(&self, package: &Package) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        if !self.enabled_sources.contains(&package.source) {
            tracing::warn!(source = ?package.source, "Attempted to install from disabled source");
            anyhow::bail!("{} source is disabled. Enable it in settings to install packages from this source.", package.source);
        }

        if let Some(backend) = self.backends.get(&package.source) {
            tracing::info!(package = %package.name, source = ?package.source, "Installing package");
            match backend.install(&package.name).await {
                Ok(()) => {
                    tracing::info!(package = %package.name, source = ?package.source, "Package installed successfully");
                    Ok(())
                }
                Err(e) => {
                    tracing::error!(package = %package.name, source = ?package.source, error = %e, "Failed to install package");
                    Err(e)
                }
            }
        } else {
            tracing::error!(source = ?package.source, "No backend available for source");
            anyhow::bail!("No backend available for {}. This package source may not be installed on your system.", package.source)
        }
    }

    pub async fn install_streaming(
        &self,
        package: &Package,
        log_sender: Option<mpsc::Sender<StreamLine>>,
    ) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        if !self.enabled_sources.contains(&package.source) {
            tracing::warn!(source = ?package.source, "Attempted to install from disabled source");
            anyhow::bail!("{} source is disabled. Enable it in settings to install packages from this source.", package.source);
        }

        if let Some(backend) = self.backends.get(&package.source) {
            tracing::info!(package = %package.name, source = ?package.source, "Installing package");
            match backend.install_streaming(&package.name, log_sender).await {
                Ok(()) => {
                    tracing::info!(package = %package.name, source = ?package.source, "Package installed successfully");
                    Ok(())
                }
                Err(e) => {
                    tracing::error!(package = %package.name, source = ?package.source, error = %e, "Failed to install package");
                    Err(e)
                }
            }
        } else {
            tracing::error!(source = ?package.source, "No backend available for source");
            anyhow::bail!("No backend available for {}. This package source may not be installed on your system.", package.source)
        }
    }

    pub async fn remove(&self, package: &Package) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        if !self.enabled_sources.contains(&package.source) {
            tracing::warn!(source = ?package.source, "Attempted to remove from disabled source");
            anyhow::bail!(
                "{} source is disabled. Enable it in settings to manage packages from this source.",
                package.source
            );
        }

        if let Some(backend) = self.backends.get(&package.source) {
            tracing::info!(package = %package.name, source = ?package.source, "Removing package");
            match backend.remove(&package.name).await {
                Ok(()) => {
                    tracing::info!(package = %package.name, source = ?package.source, "Package removed successfully");
                    Ok(())
                }
                Err(e) => {
                    tracing::error!(package = %package.name, source = ?package.source, error = %e, "Failed to remove package");
                    Err(e)
                }
            }
        } else {
            tracing::error!(source = ?package.source, "No backend available for source");
            anyhow::bail!("No backend available for {}. This package source may not be installed on your system.", package.source)
        }
    }

    pub async fn update(&self, package: &Package) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        if !self.enabled_sources.contains(&package.source) {
            tracing::warn!(source = ?package.source, "Attempted to update from disabled source");
            anyhow::bail!(
                "{} source is disabled. Enable it in settings to manage packages from this source.",
                package.source
            );
        }

        if let Some(backend) = self.backends.get(&package.source) {
            tracing::info!(package = %package.name, source = ?package.source, "Updating package");
            match backend.update(&package.name).await {
                Ok(()) => {
                    tracing::info!(package = %package.name, source = ?package.source, "Package updated successfully");
                    Ok(())
                }
                Err(e) => {
                    tracing::error!(package = %package.name, source = ?package.source, error = %e, "Failed to update package");
                    Err(e)
                }
            }
        } else {
            tracing::error!(source = ?package.source, "No backend available for source");
            anyhow::bail!("No backend available for {}. This package source may not be installed on your system.", package.source)
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Package>> {
        use futures::future::join_all;

        tracing::debug!(query = %query, "Searching across all enabled backends");

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
                        tracing::debug!(
                            source = ?source,
                            result_count = packages.len(),
                            "Search results from backend"
                        );
                    }
                    success_count += 1;
                    all_results.extend(packages);
                }
                Err(e) => {
                    error_count += 1;
                    tracing::warn!(source = ?source, error = %e, "Search failed for backend");
                }
            }
        }

        all_results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        tracing::info!(
            query = %query,
            total_results = all_results.len(),
            successful_backends = success_count,
            failed_backends = error_count,
            "Search completed"
        );

        Ok(all_results)
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
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::new()
    }
}
