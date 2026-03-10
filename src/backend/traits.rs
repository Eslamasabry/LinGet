use super::streaming::StreamLine;
use crate::models::{Package, PackageSource, PackageStatus, Repository};
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Default)]
pub struct LockStatus {
    pub is_locked: bool,
    pub lock_holder: Option<String>,
    pub lock_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchProviderSummary {
    pub source: PackageSource,
    pub result_count: usize,
    pub surfaced_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    pub package: Package,
    pub alternative_sources: Vec<PackageSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchCatalog {
    pub query: String,
    pub matches: Vec<SearchMatch>,
    pub providers: Vec<SearchProviderSummary>,
}

impl SearchCatalog {
    pub fn packages(&self) -> Vec<Package> {
        self.matches
            .iter()
            .map(|entry| entry.package.clone())
            .collect()
    }

    pub fn represented_provider_count(&self) -> usize {
        self.providers
            .iter()
            .filter(|provider| provider.result_count > 0 && provider.error.is_none())
            .count()
    }

    pub fn total_result_count(&self) -> usize {
        self.providers
            .iter()
            .map(|provider| provider.result_count)
            .sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendCapability {
    Install,
    Remove,
    Update,
    Search,
    Changelog,
    Downgrade,
    DowngradeToVersion,
    AvailableDowngradeVersions,
    ListRepositories,
    AddRepository,
    RemoveRepository,
    CleanupCache,
    ListOrphanedPackages,
    ReverseDependencies,
    PackageCommands,
    CheckLockStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapabilityStatus {
    reason: Option<String>,
}

impl CapabilityStatus {
    pub fn supported() -> Self {
        Self { reason: None }
    }

    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            reason: Some(reason.into()),
        }
    }

    pub fn is_supported(&self) -> bool {
        self.reason.is_none()
    }

    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendCapabilities {
    source: PackageSource,
}

impl BackendCapabilities {
    pub fn for_source(source: PackageSource) -> Self {
        Self { source }
    }

    pub fn status(&self, capability: BackendCapability) -> CapabilityStatus {
        let supported = match capability {
            BackendCapability::Install
            | BackendCapability::Remove
            | BackendCapability::Update
            | BackendCapability::Search => true,
            BackendCapability::Changelog => matches!(
                self.source,
                PackageSource::Apt
                    | PackageSource::Dnf
                    | PackageSource::Flatpak
                    | PackageSource::Pip
                    | PackageSource::Npm
                    | PackageSource::Cargo
                    | PackageSource::Conda
                    | PackageSource::Mamba
                    | PackageSource::Brew
            ),
            BackendCapability::Downgrade => {
                matches!(self.source, PackageSource::Dnf | PackageSource::Snap)
            }
            BackendCapability::DowngradeToVersion => matches!(
                self.source,
                PackageSource::Apt
                    | PackageSource::Dnf
                    | PackageSource::Flatpak
                    | PackageSource::Pip
                    | PackageSource::Pipx
                    | PackageSource::Cargo
                    | PackageSource::Npm
                    | PackageSource::Conda
                    | PackageSource::Mamba
                    | PackageSource::Dart
            ),
            BackendCapability::AvailableDowngradeVersions => matches!(
                self.source,
                PackageSource::Apt
                    | PackageSource::Dnf
                    | PackageSource::Flatpak
                    | PackageSource::Pip
                    | PackageSource::Cargo
                    | PackageSource::Npm
            ),
            BackendCapability::ListRepositories
            | BackendCapability::AddRepository
            | BackendCapability::RemoveRepository => matches!(
                self.source,
                PackageSource::Apt | PackageSource::Dnf | PackageSource::Flatpak
            ),
            BackendCapability::CleanupCache | BackendCapability::ListOrphanedPackages => matches!(
                self.source,
                PackageSource::Apt
                    | PackageSource::Dnf
                    | PackageSource::Flatpak
                    | PackageSource::Snap
                    | PackageSource::Pacman
                    | PackageSource::Pip
                    | PackageSource::Npm
            ),
            BackendCapability::ReverseDependencies => matches!(
                self.source,
                PackageSource::Apt
                    | PackageSource::Dnf
                    | PackageSource::Flatpak
                    | PackageSource::Pacman
                    | PackageSource::Zypper
            ),
            BackendCapability::PackageCommands => matches!(
                self.source,
                PackageSource::Apt
                    | PackageSource::Flatpak
                    | PackageSource::Snap
                    | PackageSource::Pip
                    | PackageSource::Pipx
                    | PackageSource::Cargo
                    | PackageSource::Npm
                    | PackageSource::Dart
                    | PackageSource::AppImage
            ),
            BackendCapability::CheckLockStatus => {
                matches!(self.source, PackageSource::Apt | PackageSource::Dnf)
            }
        };

        if supported {
            CapabilityStatus::supported()
        } else {
            CapabilityStatus::unsupported(self.unsupported_message(capability))
        }
    }

    fn unsupported_message(&self, capability: BackendCapability) -> String {
        match capability {
            BackendCapability::Install => "Install is not supported for this source".to_string(),
            BackendCapability::Remove => "Remove is not supported for this source".to_string(),
            BackendCapability::Update => "Update is not supported for this source".to_string(),
            BackendCapability::Search => "Search is not supported for this source".to_string(),
            BackendCapability::Changelog => {
                "Changelog is not supported for this source yet".to_string()
            }
            BackendCapability::Downgrade => {
                "Downgrade is not supported for this source".to_string()
            }
            BackendCapability::DowngradeToVersion => {
                "Downgrading to a specific version is not supported for this source".to_string()
            }
            BackendCapability::AvailableDowngradeVersions => {
                "Version history is not available for this source".to_string()
            }
            BackendCapability::ListRepositories => {
                "Repository listing is not supported for this source".to_string()
            }
            BackendCapability::AddRepository => {
                "Adding repositories is not supported for this source".to_string()
            }
            BackendCapability::RemoveRepository => {
                "Removing repositories is not supported for this source".to_string()
            }
            BackendCapability::CleanupCache => {
                "Cache cleanup is not supported for this source".to_string()
            }
            BackendCapability::ListOrphanedPackages => {
                "Orphaned package inspection is not supported for this source".to_string()
            }
            BackendCapability::ReverseDependencies => {
                "Reverse dependency inspection is not supported for this source".to_string()
            }
            BackendCapability::PackageCommands => {
                "Executable command inspection is not supported for this source".to_string()
            }
            BackendCapability::CheckLockStatus => {
                "Lock status inspection is not supported for this source".to_string()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCapabilityContext {
    source: PackageSource,
    backend_available: bool,
    source_enabled: bool,
}

impl SourceCapabilityContext {
    pub fn new(source: PackageSource, backend_available: bool, source_enabled: bool) -> Self {
        Self {
            source,
            backend_available,
            source_enabled,
        }
    }

    pub fn available(source: PackageSource) -> Self {
        Self::new(source, true, true)
    }

    pub fn status(&self, capability: BackendCapability) -> CapabilityStatus {
        if !self.source_enabled {
            return CapabilityStatus::unsupported(format!(
                "{} source is disabled. Enable it in settings to use this action.",
                self.source
            ));
        }

        if !self.backend_available {
            return CapabilityStatus::unsupported(format!(
                "No backend available for {}. This package source may not be installed on your system.",
                self.source
            ));
        }

        BackendCapabilities::for_source(self.source).status(capability)
    }

    pub fn package_status(
        &self,
        package: &Package,
        capability: BackendCapability,
    ) -> CapabilityStatus {
        let status = self.status(capability);
        if !status.is_supported() {
            return status;
        }

        match capability {
            BackendCapability::Install => {
                if package.status == PackageStatus::NotInstalled {
                    CapabilityStatus::supported()
                } else {
                    CapabilityStatus::unsupported(format!("{} is already installed", package.name))
                }
            }
            BackendCapability::Remove => {
                if matches!(
                    package.status,
                    PackageStatus::Installed | PackageStatus::UpdateAvailable
                ) {
                    CapabilityStatus::supported()
                } else {
                    CapabilityStatus::unsupported(format!("{} is not installed", package.name))
                }
            }
            BackendCapability::Update => {
                if package.status == PackageStatus::UpdateAvailable {
                    CapabilityStatus::supported()
                } else {
                    CapabilityStatus::unsupported(format!(
                        "{} has no update available",
                        package.name
                    ))
                }
            }
            BackendCapability::Downgrade
            | BackendCapability::DowngradeToVersion
            | BackendCapability::AvailableDowngradeVersions => {
                if matches!(
                    package.status,
                    PackageStatus::Installed | PackageStatus::UpdateAvailable
                ) {
                    CapabilityStatus::supported()
                } else {
                    CapabilityStatus::unsupported(format!("{} is not installed", package.name))
                }
            }
            _ => CapabilityStatus::supported(),
        }
    }
}

/// Trait that all package manager backends must implement
#[async_trait]
pub trait PackageBackend: Send + Sync {
    /// Check if this backend is available on the system
    fn is_available() -> bool
    where
        Self: Sized;

    /// List all installed packages
    async fn list_installed(&self) -> Result<Vec<Package>>;

    /// Check for available updates
    async fn check_updates(&self) -> Result<Vec<Package>>;

    /// Install a package by name
    async fn install(&self, name: &str) -> Result<()>;

    async fn install_streaming(
        &self,
        name: &str,
        _log_sender: Option<mpsc::Sender<StreamLine>>,
    ) -> Result<()> {
        self.install(name).await
    }

    /// Remove a package by name
    async fn remove(&self, name: &str) -> Result<()>;

    async fn remove_streaming(
        &self,
        name: &str,
        _log_sender: Option<mpsc::Sender<StreamLine>>,
    ) -> Result<()> {
        self.remove(name).await
    }

    /// Update a package by name
    async fn update(&self, name: &str) -> Result<()>;

    async fn update_streaming(
        &self,
        name: &str,
        _log_sender: Option<mpsc::Sender<StreamLine>>,
    ) -> Result<()> {
        self.update(name).await
    }

    /// Downgrade/revert a package by name (best-effort; optional per backend)
    async fn downgrade(&self, _name: &str) -> Result<()> {
        anyhow::bail!("Downgrade is not supported for this source")
    }

    #[allow(dead_code)]
    async fn downgrade_to(&self, _name: &str, _version: &str) -> Result<()> {
        anyhow::bail!("Downgrade to a specific version is not supported for this source")
    }

    #[allow(dead_code)]
    async fn available_downgrade_versions(&self, _name: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// Get changelog/release notes for a package (optional per backend)
    /// Returns markdown-formatted changelog if available
    async fn get_changelog(&self, _name: &str) -> Result<Option<String>> {
        Ok(None)
    }

    /// List configured repositories (optional per backend)
    async fn list_repositories(&self) -> Result<Vec<Repository>> {
        Ok(Vec::new())
    }

    #[allow(dead_code)]
    async fn add_repository(&self, _url: &str, _name: Option<&str>) -> Result<()> {
        anyhow::bail!("Adding repositories is not supported for this source")
    }

    #[allow(dead_code)]
    async fn remove_repository(&self, _name: &str) -> Result<()> {
        anyhow::bail!("Removing repositories is not supported for this source")
    }

    /// Search for new packages
    async fn search(&self, query: &str) -> Result<Vec<Package>>;

    /// Get the size of the package cache in bytes (for cleanup tools)
    async fn get_cache_size(&self) -> Result<u64> {
        Ok(0)
    }

    /// Get packages that are no longer needed (orphaned dependencies)
    async fn get_orphaned_packages(&self) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }

    /// Clean up package cache, returns bytes freed
    async fn cleanup_cache(&self) -> Result<u64> {
        Ok(0)
    }

    /// Get packages that depend on the given package (reverse dependencies)
    async fn get_reverse_dependencies(&self, _name: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    #[allow(dead_code)]
    fn source(&self) -> PackageSource;

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::for_source(self.source())
    }

    async fn get_package_commands(&self, _name: &str) -> Result<Vec<(String, PathBuf)>> {
        Ok(Vec::new())
    }

    async fn check_lock_status(&self) -> LockStatus {
        LockStatus::default()
    }
}

// We need async_trait for async trait methods
#[macro_export]
macro_rules! impl_async_trait {
    () => {
        use async_trait::async_trait;
    };
}

#[cfg(test)]
mod tests {
    use super::{BackendCapabilities, BackendCapability, SourceCapabilityContext};
    use crate::models::{Package, PackageSource, PackageStatus};

    fn make_pkg(name: &str, source: PackageSource, status: PackageStatus) -> Package {
        Package {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            available_version: Some("1.1.0".to_string()),
            description: String::new(),
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

    #[test]
    fn changelog_support_matches_current_backend_matrix() {
        assert!(BackendCapabilities::for_source(PackageSource::Apt)
            .status(BackendCapability::Changelog)
            .is_supported());
        assert!(BackendCapabilities::for_source(PackageSource::Dnf)
            .status(BackendCapability::Changelog)
            .is_supported());
        assert!(BackendCapabilities::for_source(PackageSource::Cargo)
            .status(BackendCapability::Changelog)
            .is_supported());
        assert!(BackendCapabilities::for_source(PackageSource::Flatpak)
            .status(BackendCapability::Changelog)
            .is_supported());
        assert!(BackendCapabilities::for_source(PackageSource::Brew)
            .status(BackendCapability::Changelog)
            .is_supported());
        assert!(!BackendCapabilities::for_source(PackageSource::Snap)
            .status(BackendCapability::Changelog)
            .is_supported());
    }

    #[test]
    fn package_capability_status_includes_package_state() {
        let context = SourceCapabilityContext::available(PackageSource::Apt);
        let installed = make_pkg("pkg", PackageSource::Apt, PackageStatus::Installed);
        let not_installed = make_pkg("pkg", PackageSource::Apt, PackageStatus::NotInstalled);

        assert_eq!(
            context
                .package_status(&installed, BackendCapability::Install)
                .reason(),
            Some("pkg is already installed")
        );
        assert_eq!(
            context
                .package_status(&not_installed, BackendCapability::Remove)
                .reason(),
            Some("pkg is not installed")
        );
    }

    #[test]
    fn source_context_reports_disabled_sources_before_backend_features() {
        let context = SourceCapabilityContext::new(PackageSource::Dnf, true, false);
        assert_eq!(
            context.status(BackendCapability::Changelog).reason(),
            Some("DNF source is disabled. Enable it in settings to use this action.")
        );
    }
}
