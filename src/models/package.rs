use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents which package manager a package belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PackageSource {
    Apt,
    Flatpak,
    Snap,
    Npm,
    Pip,
    Deb,
    AppImage,
}

impl fmt::Display for PackageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageSource::Apt => write!(f, "APT"),
            PackageSource::Flatpak => write!(f, "Flatpak"),
            PackageSource::Snap => write!(f, "Snap"),
            PackageSource::Npm => write!(f, "npm"),
            PackageSource::Pip => write!(f, "pip"),
            PackageSource::Deb => write!(f, "DEB"),
            PackageSource::AppImage => write!(f, "AppImage"),
        }
    }
}

impl PackageSource {
    pub fn icon_name(&self) -> &'static str {
        match self {
            PackageSource::Apt => "package-x-generic-symbolic",
            PackageSource::Flatpak => "system-software-install-symbolic",
            PackageSource::Snap => "snap-symbolic",
            PackageSource::Npm => "text-x-script-symbolic",
            PackageSource::Pip => "text-x-python-symbolic",
            PackageSource::Deb => "application-x-deb-symbolic",
            PackageSource::AppImage => "application-x-executable-symbolic",
        }
    }

    pub fn color_class(&self) -> &'static str {
        match self {
            PackageSource::Apt => "source-apt",
            PackageSource::Flatpak => "source-flatpak",
            PackageSource::Snap => "source-snap",
            PackageSource::Npm => "source-npm",
            PackageSource::Pip => "source-pip",
            PackageSource::Deb => "source-deb",
            PackageSource::AppImage => "source-appimage",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PackageSource::Apt => "System packages (Debian/Ubuntu)",
            PackageSource::Flatpak => "Sandboxed applications",
            PackageSource::Snap => "Snap packages (Ubuntu)",
            PackageSource::Npm => "Node.js packages (global)",
            PackageSource::Pip => "Python packages",
            PackageSource::Deb => "Local .deb packages",
            PackageSource::AppImage => "Portable AppImage applications",
        }
    }
}

/// The status of a package
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageStatus {
    Installed,
    UpdateAvailable,
    NotInstalled,
    Installing,
    Removing,
    Updating,
}

impl fmt::Display for PackageStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageStatus::Installed => write!(f, "Installed"),
            PackageStatus::UpdateAvailable => write!(f, "Update Available"),
            PackageStatus::NotInstalled => write!(f, "Not Installed"),
            PackageStatus::Installing => write!(f, "Installing..."),
            PackageStatus::Removing => write!(f, "Removing..."),
            PackageStatus::Updating => write!(f, "Updating..."),
        }
    }
}

/// Represents a software package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub available_version: Option<String>,
    pub description: String,
    pub source: PackageSource,
    pub status: PackageStatus,
    pub size: Option<u64>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub maintainer: Option<String>,
    pub dependencies: Vec<String>,
    pub install_date: Option<String>,
}

impl Package {
    pub fn has_update(&self) -> bool {
        self.status == PackageStatus::UpdateAvailable
    }

    pub fn display_version(&self) -> String {
        if let Some(ref available) = self.available_version {
            if self.has_update() {
                return format!("{} → {}", self.version, available);
            }
        }
        self.version.clone()
    }

    pub fn size_display(&self) -> String {
        match self.size {
            Some(size) => humansize::format_size(size, humansize::BINARY),
            None => String::from("Unknown"),
        }
    }

    pub fn id(&self) -> String {
        format!("{}:{}", self.source, self.name)
    }
}

impl PartialEq for Package {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.source == other.source
    }
}

impl Eq for Package {}

impl std::hash::Hash for Package {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.source.hash(state);
    }
}
