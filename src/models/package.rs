use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents which package manager a package belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PackageSource {
    Apt,
    Dnf,
    Pacman,
    Zypper,
    Flatpak,
    Snap,
    Npm,
    Pip,
    Pipx,
    Cargo,
    Brew,
    Aur,
    Conda,
    Mamba,
    Dart,
    Deb,
    AppImage,
}

impl fmt::Display for PackageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageSource::Apt => write!(f, "APT"),
            PackageSource::Dnf => write!(f, "DNF"),
            PackageSource::Pacman => write!(f, "Pacman"),
            PackageSource::Zypper => write!(f, "Zypper"),
            PackageSource::Flatpak => write!(f, "Flatpak"),
            PackageSource::Snap => write!(f, "Snap"),
            PackageSource::Npm => write!(f, "npm"),
            PackageSource::Pip => write!(f, "pip"),
            PackageSource::Pipx => write!(f, "pipx"),
            PackageSource::Cargo => write!(f, "cargo"),
            PackageSource::Brew => write!(f, "brew"),
            PackageSource::Aur => write!(f, "AUR"),
            PackageSource::Conda => write!(f, "conda"),
            PackageSource::Mamba => write!(f, "mamba"),
            PackageSource::Dart => write!(f, "dart"),
            PackageSource::Deb => write!(f, "DEB"),
            PackageSource::AppImage => write!(f, "AppImage"),
        }
    }
}

impl PackageSource {
    pub fn icon_name(&self) -> &'static str {
        match self {
            PackageSource::Apt => "package-x-generic-symbolic",
            PackageSource::Dnf => "system-software-install-symbolic",
            PackageSource::Pacman => "package-x-generic-symbolic",
            PackageSource::Zypper => "system-software-install-symbolic",
            PackageSource::Flatpak => "system-software-install-symbolic",
            PackageSource::Snap => "snap-symbolic",
            PackageSource::Npm => "text-x-script-symbolic",
            PackageSource::Pip => "text-x-python-symbolic",
            PackageSource::Pipx => "text-x-python-symbolic",
            PackageSource::Cargo => "applications-development-symbolic",
            PackageSource::Brew => "application-x-executable-symbolic",
            PackageSource::Aur => "package-x-generic-symbolic",
            PackageSource::Conda => "text-x-python-symbolic",
            PackageSource::Mamba => "text-x-python-symbolic",
            PackageSource::Dart => "applications-development-symbolic",
            PackageSource::Deb => "application-x-deb-symbolic",
            PackageSource::AppImage => "application-x-executable-symbolic",
        }
    }

    pub fn color_class(&self) -> &'static str {
        match self {
            PackageSource::Apt => "source-apt",
            PackageSource::Dnf => "source-dnf",
            PackageSource::Pacman => "source-pacman",
            PackageSource::Zypper => "source-zypper",
            PackageSource::Flatpak => "source-flatpak",
            PackageSource::Snap => "source-snap",
            PackageSource::Npm => "source-npm",
            PackageSource::Pip => "source-pip",
            PackageSource::Pipx => "source-pipx",
            PackageSource::Cargo => "source-cargo",
            PackageSource::Brew => "source-brew",
            PackageSource::Aur => "source-aur",
            PackageSource::Conda => "source-conda",
            PackageSource::Mamba => "source-mamba",
            PackageSource::Dart => "source-dart",
            PackageSource::Deb => "source-deb",
            PackageSource::AppImage => "source-appimage",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PackageSource::Apt => "System packages (Debian/Ubuntu)",
            PackageSource::Dnf => "System packages (Fedora/RHEL)",
            PackageSource::Pacman => "System packages (Arch Linux)",
            PackageSource::Zypper => "System packages (openSUSE)",
            PackageSource::Flatpak => "Sandboxed applications",
            PackageSource::Snap => "Snap packages (Ubuntu)",
            PackageSource::Npm => "Node.js packages (global)",
            PackageSource::Pip => "Python packages",
            PackageSource::Pipx => "Python app packages (pipx)",
            PackageSource::Cargo => "Rust crates (cargo install)",
            PackageSource::Brew => "Homebrew packages (Linuxbrew)",
            PackageSource::Aur => "Arch User Repository (AUR helper)",
            PackageSource::Conda => "Conda packages (base env)",
            PackageSource::Mamba => "Mamba packages (base env)",
            PackageSource::Dart => "Dart/Flutter global tools (pub global)",
            PackageSource::Deb => "Local .deb packages",
            PackageSource::AppImage => "Portable AppImage applications",
        }
    }

    /// Returns true if this source supports install/remove/update operations in the GUI
    pub fn supports_gui_operations(&self) -> bool {
        // All sources now support GUI operations
        true
    }

    /// Returns a user-friendly warning about potential risks for certain sources
    pub fn gui_operation_warning(&self) -> Option<&'static str> {
        match self {
            PackageSource::Aur => Some("AUR packages use --noconfirm mode. For sensitive packages, consider using your terminal with yay/paru to review the PKGBUILD."),
            _ => None,
        }
    }

    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "apt" => Some(PackageSource::Apt),
            "dnf" => Some(PackageSource::Dnf),
            "pacman" => Some(PackageSource::Pacman),
            "zypper" => Some(PackageSource::Zypper),
            "flatpak" => Some(PackageSource::Flatpak),
            "snap" => Some(PackageSource::Snap),
            "npm" => Some(PackageSource::Npm),
            "pip" => Some(PackageSource::Pip),
            "pipx" => Some(PackageSource::Pipx),
            "cargo" => Some(PackageSource::Cargo),
            "brew" => Some(PackageSource::Brew),
            "aur" => Some(PackageSource::Aur),
            "conda" => Some(PackageSource::Conda),
            "mamba" => Some(PackageSource::Mamba),
            "dart" => Some(PackageSource::Dart),
            "deb" => Some(PackageSource::Deb),
            "appimage" => Some(PackageSource::AppImage),
            _ => None,
        }
    }

    /// Convert to lowercase string for config storage
    pub fn as_config_str(self) -> &'static str {
        match self {
            PackageSource::Apt => "apt",
            PackageSource::Dnf => "dnf",
            PackageSource::Pacman => "pacman",
            PackageSource::Zypper => "zypper",
            PackageSource::Flatpak => "flatpak",
            PackageSource::Snap => "snap",
            PackageSource::Npm => "npm",
            PackageSource::Pip => "pip",
            PackageSource::Pipx => "pipx",
            PackageSource::Cargo => "cargo",
            PackageSource::Brew => "brew",
            PackageSource::Aur => "aur",
            PackageSource::Conda => "conda",
            PackageSource::Mamba => "mamba",
            PackageSource::Dart => "dart",
            PackageSource::Deb => "deb",
            PackageSource::AppImage => "appimage",
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
    // Enrichment fields (populated asynchronously)
    #[serde(default)]
    pub enrichment: Option<PackageEnrichment>,
}

/// Rich metadata fetched from online sources
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageEnrichment {
    /// URL to the package icon (high-res)
    pub icon_url: Option<String>,
    /// URLs to screenshots
    pub screenshots: Vec<String>,
    /// App categories (e.g., "Development", "Utilities")
    pub categories: Vec<String>,
    /// Developer or publisher name
    pub developer: Option<String>,
    /// User rating (0.0 - 5.0)
    pub rating: Option<f32>,
    /// Download/install count
    pub downloads: Option<u64>,
    /// Long-form description or summary
    pub summary: Option<String>,
    /// Project repository URL
    pub repository: Option<String>,
    /// Keywords/tags
    pub keywords: Vec<String>,
    /// Last updated timestamp
    pub last_updated: Option<String>,
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
