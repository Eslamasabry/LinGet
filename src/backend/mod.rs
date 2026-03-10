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
pub mod history_tracker;
mod mamba;
mod npm;
mod pacman;
mod pip;
mod pipx;
mod pkexec;
mod providers;
mod snap;
pub mod streaming;
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
#[allow(unused_imports)]
pub use history_tracker::HistoryTracker;
pub use mamba::MambaBackend;
pub use npm::NpmBackend;
pub use pacman::PacmanBackend;
pub use pip::PipBackend;
pub use pipx::PipxBackend;
pub use pkexec::{run_pkexec, run_pkexec_with_logs, Suggest, SUGGEST_PREFIX};
pub use providers::{detect_available_providers, detect_providers, ProviderStatus};
pub use snap::SnapBackend;
pub use traits::*;
pub use zypper::ZypperBackend;

use crate::backend::streaming::StreamLine;
use crate::models::history::{TaskQueueAction, TaskQueueEntry};
use crate::models::{
    normalize_name_for_dedup, FlatpakMetadata, FlatpakPermission, Package, PackageSource,
    PackageStatus, Repository,
};
use anyhow::{Context, Result};
#[cfg(test)]
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, instrument, warn};

#[cfg(test)]
pub(crate) static TEST_PATH_ENV_LOCK: Lazy<tokio::sync::Mutex<()>> =
    Lazy::new(|| tokio::sync::Mutex::new(()));

#[derive(Debug, Clone)]
pub enum TaskQueueEvent {
    Started(TaskQueueEntry),
    Log { entry_id: String, line: StreamLine },
    Completed(TaskQueueEntry),
    Failed(TaskQueueEntry),
}

#[derive(Clone)]
pub struct TaskQueueExecutor {
    package_manager: Arc<Mutex<PackageManager>>,
    history_tracker: Arc<Mutex<Option<HistoryTracker>>>,
}

impl TaskQueueExecutor {
    pub fn new(
        package_manager: Arc<Mutex<PackageManager>>,
        history_tracker: Arc<Mutex<Option<HistoryTracker>>>,
    ) -> Self {
        Self {
            package_manager,
            history_tracker,
        }
    }

    pub async fn run(&self, event_sender: Option<mpsc::Sender<TaskQueueEvent>>) -> Result<()> {
        loop {
            let entry = {
                let mut guard = self.history_tracker.lock().await;
                let tracker = guard
                    .as_mut()
                    .context("History tracker must be initialized to run task queue")?;
                tracker.claim_next_task().await?
            };

            let Some(entry) = entry else {
                break;
            };

            if let Some(sender) = &event_sender {
                let _ = sender.send(TaskQueueEvent::Started(entry.clone())).await;
            }

            let (log_sender, log_task) = Self::spawn_log_forwarder(&event_sender, &entry);
            let result = {
                let manager = self.package_manager.lock().await;
                let pkg = Self::package_from_entry(&entry);
                match entry.action {
                    TaskQueueAction::Install => manager.install_streaming(&pkg, log_sender).await,
                    TaskQueueAction::Remove => manager.remove_streaming(&pkg, log_sender).await,
                    TaskQueueAction::Update => manager.update_streaming(&pkg, log_sender).await,
                }
            };

            if let Some(task) = log_task {
                let _ = task.await;
            }

            match result {
                Ok(()) => {
                    let updated = {
                        let mut guard = self.history_tracker.lock().await;
                        let tracker = guard
                            .as_mut()
                            .context("History tracker missing after task completion")?;
                        tracker.mark_task_completed(&entry.id).await?
                    };

                    let completed_entry = updated.unwrap_or_else(|| {
                        let mut fallback = entry.clone();
                        fallback.mark_completed();
                        fallback
                    });

                    if let Some(sender) = &event_sender {
                        let _ = sender
                            .send(TaskQueueEvent::Completed(completed_entry))
                            .await;
                    }
                }
                Err(e) => {
                    let error = e.to_string();
                    let updated = {
                        let mut guard = self.history_tracker.lock().await;
                        let tracker = guard
                            .as_mut()
                            .context("History tracker missing after task failure")?;
                        tracker.mark_task_failed(&entry.id, error.clone()).await?
                    };

                    let failed_entry = updated.unwrap_or_else(|| {
                        let mut fallback = entry.clone();
                        fallback.mark_failed(error.clone());
                        fallback
                    });

                    if let Some(sender) = &event_sender {
                        let _ = sender.send(TaskQueueEvent::Failed(failed_entry)).await;
                    }
                }
            }
        }

        Ok(())
    }

    fn spawn_log_forwarder(
        event_sender: &Option<mpsc::Sender<TaskQueueEvent>>,
        entry: &TaskQueueEntry,
    ) -> (
        Option<mpsc::Sender<StreamLine>>,
        Option<tokio::task::JoinHandle<()>>,
    ) {
        let Some(sender) = event_sender.as_ref().cloned() else {
            return (None, None);
        };

        let (log_tx, mut log_rx) = mpsc::channel(200);
        let entry_id = entry.id.clone();
        let log_task = tokio::spawn(async move {
            while let Some(line) = log_rx.recv().await {
                let _ = sender
                    .send(TaskQueueEvent::Log {
                        entry_id: entry_id.clone(),
                        line,
                    })
                    .await;
            }
        });

        (Some(log_tx), Some(log_task))
    }

    fn package_from_entry(entry: &TaskQueueEntry) -> Package {
        let status = match entry.action {
            TaskQueueAction::Install => PackageStatus::NotInstalled,
            TaskQueueAction::Remove => PackageStatus::Installed,
            TaskQueueAction::Update => PackageStatus::UpdateAvailable,
        };

        Package {
            name: entry.package_name.clone(),
            version: String::new(),
            available_version: None,
            description: String::new(),
            source: entry.package_source,
            status,
            size: None,
            homepage: None,
            license: None,
            maintainer: None,
            dependencies: Vec::new(),
            install_date: None,
            update_category: None,
            enrichment: None,
        }
    }
}

/// Manager that coordinates all package backends
pub struct PackageManager {
    backends: HashMap<PackageSource, Box<dyn PackageBackend>>,
    enabled_sources: HashSet<PackageSource>,
    provider_statuses: HashMap<PackageSource, ProviderStatus>,
}

impl PackageManager {
    pub fn new() -> Self {
        info!("Initializing PackageManager, detecting available backends");
        let provider_statuses = detect_providers()
            .into_iter()
            .map(|status| (status.source, status))
            .collect();
        let mut backends: HashMap<PackageSource, Box<dyn PackageBackend>> = HashMap::new();

        // Add available backends with logging
        let mut check_backend =
            |source: PackageSource, available: bool, backend: Box<dyn PackageBackend>| {
                if available {
                    debug!(source = ?source, "Backend available");
                    backends.insert(source, backend);
                } else {
                    debug!(source = ?source, "Backend not available");
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
        info!(
            available_backends = backends.len(),
            backends = ?backends.keys().collect::<Vec<_>>(),
            "PackageManager initialized"
        );

        Self {
            backends,
            enabled_sources,
            provider_statuses,
        }
    }

    #[allow(dead_code)]
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

    pub fn get_backend(&self, source: PackageSource) -> Option<&dyn PackageBackend> {
        self.backends.get(&source).map(|b| b.as_ref())
    }

    pub fn source_capability_status(
        &self,
        source: PackageSource,
        capability: BackendCapability,
    ) -> CapabilityStatus {
        if !self.enabled_sources.contains(&source) {
            return SourceCapabilityContext::new(source, true, false).status(capability);
        }

        if let Some(backend) = self.backends.get(&source) {
            backend.capabilities().status(capability)
        } else {
            CapabilityStatus::unsupported(self.backend_unavailable_reason(source))
        }
    }

    pub fn package_capability_status(
        &self,
        package: &Package,
        capability: BackendCapability,
    ) -> CapabilityStatus {
        if !self.enabled_sources.contains(&package.source) {
            return SourceCapabilityContext::new(package.source, true, false)
                .package_status(package, capability);
        }

        let Some(backend) = self.backends.get(&package.source) else {
            return CapabilityStatus::unsupported(self.backend_unavailable_reason(package.source));
        };

        let source_status = backend.capabilities().status(capability);
        if !source_status.is_supported() {
            return source_status;
        }

        SourceCapabilityContext::available(package.source).package_status(package, capability)
    }

    fn ensure_source_capability(
        &self,
        source: PackageSource,
        capability: BackendCapability,
    ) -> Result<()> {
        let status = self.source_capability_status(source, capability);
        match status.reason() {
            Some(reason) => anyhow::bail!(reason.to_string()),
            None => Ok(()),
        }
    }

    fn ensure_package_capability(
        &self,
        package: &Package,
        capability: BackendCapability,
    ) -> Result<()> {
        let status = self.package_capability_status(package, capability);
        match status.reason() {
            Some(reason) => anyhow::bail!(reason.to_string()),
            None => Ok(()),
        }
    }

    fn backend_unavailable_reason(&self, source: PackageSource) -> String {
        if !source.supported_on_current_platform() {
            return format!(
                "{} is only supported on {}. {}",
                source,
                source.platform_family().label(),
                source
                    .install_hint()
                    .unwrap_or("This provider is not available on the current platform.")
            );
        }

        let mut reason = format!(
            "No backend available for {}. This package source may not be installed on your system.",
            source
        );

        if let Some(status) = self.provider_statuses.get(&source) {
            if let Some(status_reason) = status.reason.as_deref() {
                reason.push(' ');
                reason.push_str(status_reason);
                reason.push('.');
            }
        }

        if let Some(hint) = source.install_hint() {
            reason.push(' ');
            reason.push_str(hint);
        }

        reason
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
        debug!(
            enabled_backends = enabled_count,
            "Listing installed packages from all enabled backends"
        );

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
        self.ensure_package_capability(package, BackendCapability::Install)?;

        let backend = self
            .backends
            .get(&package.source)
            .context("Install capability check should guarantee backend availability")?;

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
    }

    #[instrument(skip(self), fields(package = %package.name, source = ?package.source))]
    pub async fn install_streaming(
        &self,
        package: &Package,
        log_sender: Option<mpsc::Sender<StreamLine>>,
    ) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        self.ensure_package_capability(package, BackendCapability::Install)?;

        let backend = self
            .backends
            .get(&package.source)
            .context("Install capability check should guarantee backend availability")?;

        info!(package = %package.name, source = ?package.source, "Installing package");
        match backend.install_streaming(&package.name, log_sender).await {
            Ok(()) => {
                info!(package = %package.name, source = ?package.source, "Package installed successfully");
                Ok(())
            }
            Err(e) => {
                error!(package = %package.name, source = ?package.source, error = %e, "Failed to install package");
                Err(e)
            }
        }
    }

    #[instrument(skip(self), fields(package = %package.name, source = ?package.source))]
    pub async fn remove(&self, package: &Package) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        self.ensure_package_capability(package, BackendCapability::Remove)?;

        let backend = self
            .backends
            .get(&package.source)
            .context("Remove capability check should guarantee backend availability")?;

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
    }

    #[instrument(skip(self), fields(package = %package.name, source = ?package.source))]
    pub async fn remove_streaming(
        &self,
        package: &Package,
        log_sender: Option<mpsc::Sender<StreamLine>>,
    ) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        self.ensure_package_capability(package, BackendCapability::Remove)?;

        let backend = self
            .backends
            .get(&package.source)
            .context("Remove capability check should guarantee backend availability")?;

        info!(package = %package.name, source = ?package.source, "Removing package");
        match backend.remove_streaming(&package.name, log_sender).await {
            Ok(()) => {
                info!(package = %package.name, source = ?package.source, "Package removed successfully");
                Ok(())
            }
            Err(e) => {
                error!(package = %package.name, source = ?package.source, error = %e, "Failed to remove package");
                Err(e)
            }
        }
    }

    #[instrument(skip(self), fields(package = %package.name, source = ?package.source))]
    pub async fn update(&self, package: &Package) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        self.ensure_package_capability(package, BackendCapability::Update)?;

        let backend = self
            .backends
            .get(&package.source)
            .context("Update capability check should guarantee backend availability")?;

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
    }

    #[instrument(skip(self), fields(package = %package.name, source = ?package.source))]
    pub async fn update_streaming(
        &self,
        package: &Package,
        log_sender: Option<mpsc::Sender<StreamLine>>,
    ) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        self.ensure_package_capability(package, BackendCapability::Update)?;

        let backend = self
            .backends
            .get(&package.source)
            .context("Update capability check should guarantee backend availability")?;

        info!(package = %package.name, source = ?package.source, "Updating package");
        match backend.update_streaming(&package.name, log_sender).await {
            Ok(()) => {
                info!(package = %package.name, source = ?package.source, "Package updated successfully");
                Ok(())
            }
            Err(e) => {
                error!(package = %package.name, source = ?package.source, error = %e, "Failed to update package");
                Err(e)
            }
        }
    }

    pub async fn downgrade(&self, package: &Package) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        self.ensure_package_capability(package, BackendCapability::Downgrade)?;

        let backend = self
            .backends
            .get(&package.source)
            .context("Downgrade capability check should guarantee backend availability")?;
        backend.downgrade(&package.name).await
    }

    #[allow(dead_code)]
    pub async fn downgrade_to(&self, package: &Package, version: &str) -> Result<()> {
        Self::validate_package_name(&package.name)?;
        self.ensure_package_capability(package, BackendCapability::DowngradeToVersion)?;

        let backend = self
            .backends
            .get(&package.source)
            .context("Version downgrade capability check should guarantee backend availability")?;
        backend.downgrade_to(&package.name, version).await
    }

    #[allow(dead_code)]
    pub async fn available_downgrade_versions(&self, package: &Package) -> Result<Vec<String>> {
        Self::validate_package_name(&package.name)?;
        self.ensure_package_capability(package, BackendCapability::AvailableDowngradeVersions)?;

        let backend = self
            .backends
            .get(&package.source)
            .context("Downgrade history capability check should guarantee backend availability")?;
        backend.available_downgrade_versions(&package.name).await
    }

    pub async fn get_changelog(&self, package: &Package) -> Result<Option<String>> {
        Self::validate_package_name(&package.name)?;
        self.ensure_package_capability(package, BackendCapability::Changelog)?;

        let backend = self
            .backends
            .get(&package.source)
            .context("Changelog capability check should guarantee backend availability")?;
        backend.get_changelog(&package.name).await
    }

    pub async fn get_reverse_dependencies(&self, package: &Package) -> Result<Vec<String>> {
        Self::validate_package_name(&package.name)?;
        self.ensure_package_capability(package, BackendCapability::ReverseDependencies)?;

        let backend = self
            .backends
            .get(&package.source)
            .context("Reverse dependency capability check should guarantee backend availability")?;
        backend.get_reverse_dependencies(&package.name).await
    }

    #[allow(dead_code)]
    pub async fn list_repositories(&self, source: PackageSource) -> Result<Vec<Repository>> {
        self.ensure_source_capability(source, BackendCapability::ListRepositories)?;

        let backend = self
            .backends
            .get(&source)
            .context("Repository listing capability check should guarantee backend availability")?;
        backend.list_repositories().await
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

    #[allow(dead_code)]
    pub async fn add_repository(
        &self,
        source: PackageSource,
        url: &str,
        name: Option<&str>,
    ) -> Result<()> {
        self.ensure_source_capability(source, BackendCapability::AddRepository)?;

        let backend = self
            .backends
            .get(&source)
            .context("Repository add capability check should guarantee backend availability")?;
        backend.add_repository(url, name).await
    }

    #[allow(dead_code)]
    pub async fn remove_repository(&self, source: PackageSource, name: &str) -> Result<()> {
        self.ensure_source_capability(source, BackendCapability::RemoveRepository)?;

        let backend = self
            .backends
            .get(&source)
            .context("Repository removal capability check should guarantee backend availability")?;
        backend.remove_repository(name).await
    }

    async fn collect_search_results(
        &self,
        query: &str,
    ) -> Vec<(PackageSource, Result<Vec<Package>, anyhow::Error>)> {
        use futures::future::join_all;

        let futures: Vec<_> = self
            .enabled_backends()
            .filter(|(_, backend)| {
                backend
                    .capabilities()
                    .status(BackendCapability::Search)
                    .is_supported()
            })
            .map(|(source, backend)| {
                let source = *source;
                async move { (source, backend.search(query).await) }
            })
            .collect();

        join_all(futures).await
    }

    fn primary_search_package(packages: &[Package]) -> Option<Package> {
        packages
            .iter()
            .min_by(|left, right| {
                let left_status_rank = matches!(
                    left.status,
                    PackageStatus::Installed
                        | PackageStatus::UpdateAvailable
                        | PackageStatus::Updating
                );
                let right_status_rank = matches!(
                    right.status,
                    PackageStatus::Installed
                        | PackageStatus::UpdateAvailable
                        | PackageStatus::Updating
                );

                right_status_rank
                    .cmp(&left_status_rank)
                    .then_with(|| {
                        left.source
                            .discovery_priority()
                            .cmp(&right.source.discovery_priority())
                    })
                    .then_with(|| left.name.len().cmp(&right.name.len()))
                    .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            })
            .cloned()
    }

    fn build_search_catalog(
        query: &str,
        results: Vec<(PackageSource, Result<Vec<Package>>)>,
    ) -> SearchCatalog {
        let mut providers = Vec::new();
        let mut raw_packages = Vec::new();

        for (source, result) in results {
            match result {
                Ok(packages) => {
                    if !packages.is_empty() {
                        debug!(
                            source = ?source,
                            result_count = packages.len(),
                            "Search results from backend"
                        );
                    }

                    providers.push(SearchProviderSummary {
                        source,
                        result_count: packages.len(),
                        surfaced_count: 0,
                        error: None,
                    });
                    raw_packages.extend(packages);
                }
                Err(error) => {
                    warn!(source = ?source, error = %error, "Search failed for backend");
                    providers.push(SearchProviderSummary {
                        source,
                        result_count: 0,
                        surfaced_count: 0,
                        error: Some(error.to_string()),
                    });
                }
            }
        }

        let mut grouped: HashMap<String, Vec<Package>> = HashMap::new();
        for package in raw_packages {
            let normalized = normalize_name_for_dedup(&package.name, package.source);
            grouped.entry(normalized).or_default().push(package);
        }

        let mut matches = Vec::new();
        for (_normalized_name, group) in grouped {
            let Some(mut primary) = Self::primary_search_package(&group) else {
                continue;
            };

            let mut alternative_sources: Vec<_> = group
                .iter()
                .map(|package| package.source)
                .filter(|source| *source != primary.source)
                .collect();
            alternative_sources.sort_by_key(|source| source.discovery_priority());
            alternative_sources.dedup();

            if primary.description.trim().is_empty() {
                if let Some(best_description) = group
                    .iter()
                    .filter(|package| !package.description.trim().is_empty())
                    .max_by_key(|package| package.description.len())
                {
                    primary.description = best_description.description.clone();
                }
            }

            if primary.homepage.is_none() {
                primary.homepage = group.iter().find_map(|package| package.homepage.clone());
            }
            if primary.license.is_none() {
                primary.license = group.iter().find_map(|package| package.license.clone());
            }
            if primary.maintainer.is_none() {
                primary.maintainer = group.iter().find_map(|package| package.maintainer.clone());
            }

            for provider in &mut providers {
                if provider.source == primary.source
                    || alternative_sources.contains(&provider.source)
                {
                    provider.surfaced_count += 1;
                }
            }

            matches.push(SearchMatch {
                package: primary,
                alternative_sources,
            });
        }

        matches.sort_by(|left, right| {
            left.package
                .name
                .to_lowercase()
                .cmp(&right.package.name.to_lowercase())
                .then_with(|| {
                    left.package
                        .source
                        .discovery_priority()
                        .cmp(&right.package.source.discovery_priority())
                })
        });
        providers.sort_by_key(|provider| provider.source.discovery_priority());

        SearchCatalog {
            query: query.to_string(),
            matches,
            providers,
        }
    }

    #[instrument(skip(self), fields(query = %query))]
    pub async fn search_catalog(&self, query: &str) -> Result<SearchCatalog> {
        debug!(query = %query, "Searching provider catalog across enabled backends");
        let results = self.collect_search_results(query).await;
        let catalog = Self::build_search_catalog(query, results);

        info!(
            query = %query,
            total_results = catalog.matches.len(),
            raw_results = catalog.total_result_count(),
            represented_sources = catalog.represented_provider_count(),
            "Provider catalog search completed"
        );

        Ok(catalog)
    }

    #[instrument(skip(self), fields(query = %query))]
    pub async fn search(&self, query: &str) -> Result<Vec<Package>> {
        debug!(query = %query, "Searching across all enabled backends");

        let results = self.collect_search_results(query).await;
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

    pub async fn get_package_commands(
        &self,
        name: &str,
        source: PackageSource,
    ) -> Result<Vec<(String, std::path::PathBuf)>> {
        self.ensure_source_capability(source, BackendCapability::PackageCommands)?;

        let backend = self
            .backends
            .get(&source)
            .context("Package commands capability check should guarantee backend availability")?;

        tracing::info!(package = %name, source = ?source, "Calling backend.get_package_commands");
        let result = backend.get_package_commands(name).await;
        tracing::info!(
            package = %name,
            source = ?source,
            success = result.is_ok(),
            command_count = result.as_ref().map(|c| c.len()).unwrap_or(0),
            "Backend returned package commands"
        );

        result
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

    pub async fn check_all_lock_status(&self) -> Vec<(PackageSource, LockStatus)> {
        use futures::future::join_all;

        let futures: Vec<_> = self
            .backends
            .iter()
            .filter(|(_, backend)| {
                backend
                    .capabilities()
                    .status(BackendCapability::CheckLockStatus)
                    .is_supported()
            })
            .map(|(source, backend)| {
                let source = *source;
                async move { (source, backend.check_lock_status().await) }
            })
            .collect();

        let results = join_all(futures).await;
        results
            .into_iter()
            .filter(|(_, status)| status.is_locked)
            .collect()
    }
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;

    struct FakeSearchBackend {
        source: PackageSource,
        packages: Vec<Package>,
        fail: bool,
    }

    impl FakeSearchBackend {
        fn new(source: PackageSource, packages: Vec<Package>) -> Self {
            Self {
                source,
                packages,
                fail: false,
            }
        }

        fn failing(source: PackageSource) -> Self {
            Self {
                source,
                packages: Vec::new(),
                fail: true,
            }
        }
    }

    #[async_trait]
    impl PackageBackend for FakeSearchBackend {
        fn is_available() -> bool
        where
            Self: Sized,
        {
            true
        }

        async fn list_installed(&self) -> Result<Vec<Package>> {
            Ok(Vec::new())
        }

        async fn check_updates(&self) -> Result<Vec<Package>> {
            Ok(Vec::new())
        }

        async fn install(&self, _name: &str) -> Result<()> {
            Ok(())
        }

        async fn remove(&self, _name: &str) -> Result<()> {
            Ok(())
        }

        async fn update(&self, _name: &str) -> Result<()> {
            Ok(())
        }

        async fn search(&self, _query: &str) -> Result<Vec<Package>> {
            if self.fail {
                anyhow::bail!("backend unavailable")
            } else {
                Ok(self.packages.clone())
            }
        }

        fn source(&self) -> PackageSource {
            self.source
        }
    }

    fn search_package(name: &str, source: PackageSource, status: PackageStatus) -> Package {
        Package {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            available_version: None,
            description: format!("{name} package"),
            source,
            status,
            size: None,
            homepage: None,
            license: None,
            maintainer: None,
            dependencies: Vec::new(),
            install_date: None,
            update_category: None,
            enrichment: None,
        }
    }

    fn test_package_manager(
        backends: Vec<(PackageSource, Box<dyn PackageBackend>)>,
    ) -> PackageManager {
        let enabled_sources = backends.iter().map(|(source, _)| *source).collect();
        let backends = backends.into_iter().collect();
        PackageManager {
            backends,
            enabled_sources,
            provider_statuses: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn search_catalog_deduplicates_cross_source_matches() {
        let manager = test_package_manager(vec![
            (
                PackageSource::Apt,
                Box::new(FakeSearchBackend::new(
                    PackageSource::Apt,
                    vec![search_package(
                        "firefox",
                        PackageSource::Apt,
                        PackageStatus::NotInstalled,
                    )],
                )),
            ),
            (
                PackageSource::Flatpak,
                Box::new(FakeSearchBackend::new(
                    PackageSource::Flatpak,
                    vec![search_package(
                        "org.mozilla.firefox",
                        PackageSource::Flatpak,
                        PackageStatus::NotInstalled,
                    )],
                )),
            ),
        ]);

        let catalog = manager
            .search_catalog("firefox")
            .await
            .expect("catalog search");

        assert_eq!(catalog.matches.len(), 1);
        assert_eq!(catalog.matches[0].package.source, PackageSource::Apt);
        assert_eq!(
            catalog.matches[0].alternative_sources,
            vec![PackageSource::Flatpak]
        );
        assert_eq!(catalog.total_result_count(), 2);
    }

    #[tokio::test]
    async fn search_catalog_prefers_installed_match_over_source_priority() {
        let manager = test_package_manager(vec![
            (
                PackageSource::Apt,
                Box::new(FakeSearchBackend::new(
                    PackageSource::Apt,
                    vec![search_package(
                        "demo",
                        PackageSource::Apt,
                        PackageStatus::NotInstalled,
                    )],
                )),
            ),
            (
                PackageSource::Npm,
                Box::new(FakeSearchBackend::new(
                    PackageSource::Npm,
                    vec![search_package(
                        "demo",
                        PackageSource::Npm,
                        PackageStatus::Installed,
                    )],
                )),
            ),
        ]);

        let catalog = manager
            .search_catalog("demo")
            .await
            .expect("catalog search");

        assert_eq!(catalog.matches.len(), 1);
        assert_eq!(catalog.matches[0].package.source, PackageSource::Npm);
        assert_eq!(
            catalog.matches[0].alternative_sources,
            vec![PackageSource::Apt]
        );
    }

    #[tokio::test]
    async fn search_catalog_keeps_provider_failures_in_summary() {
        let manager = test_package_manager(vec![
            (
                PackageSource::Apt,
                Box::new(FakeSearchBackend::new(
                    PackageSource::Apt,
                    vec![search_package(
                        "ripgrep",
                        PackageSource::Apt,
                        PackageStatus::NotInstalled,
                    )],
                )),
            ),
            (
                PackageSource::Snap,
                Box::new(FakeSearchBackend::failing(PackageSource::Snap)),
            ),
        ]);

        let catalog = manager.search_catalog("rg").await.expect("catalog search");

        assert_eq!(catalog.matches.len(), 1);
        assert_eq!(catalog.represented_provider_count(), 1);
        assert!(catalog
            .providers
            .iter()
            .any(|provider| provider.source == PackageSource::Snap
                && provider.error.as_deref() == Some("backend unavailable")));
    }
}
