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
    pub const ALL: [PackageSource; 17] = [
        PackageSource::Apt,
        PackageSource::Dnf,
        PackageSource::Pacman,
        PackageSource::Zypper,
        PackageSource::Flatpak,
        PackageSource::Snap,
        PackageSource::Npm,
        PackageSource::Pip,
        PackageSource::Pipx,
        PackageSource::Cargo,
        PackageSource::Brew,
        PackageSource::Aur,
        PackageSource::Conda,
        PackageSource::Mamba,
        PackageSource::Dart,
        PackageSource::Deb,
        PackageSource::AppImage,
    ];

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

    pub fn icon_name(&self) -> &'static str {
        match self {
            PackageSource::Apt => "system-software-install-symbolic",
            PackageSource::Dnf => "system-software-install-symbolic",
            PackageSource::Pacman => "system-software-install-symbolic",
            PackageSource::Zypper => "system-software-install-symbolic",
            PackageSource::Flatpak => "application-x-flatpak-symbolic",
            PackageSource::Snap => "io.snapcraft.Store",
            PackageSource::Npm => "folder-script-symbolic",
            PackageSource::Pip => "folder-script-symbolic",
            PackageSource::Pipx => "folder-script-symbolic",
            PackageSource::Cargo => "folder-script-symbolic",
            PackageSource::Brew => "utilities-terminal-symbolic",
            PackageSource::Aur => "system-software-install-symbolic",
            PackageSource::Conda => "folder-script-symbolic",
            PackageSource::Mamba => "folder-script-symbolic",
            PackageSource::Dart => "folder-script-symbolic",
            PackageSource::Deb => "application-x-deb-symbolic",
            PackageSource::AppImage => "application-x-executable-symbolic",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UpdateCategory {
    Security,
    Bugfix,
    Feature,
    #[default]
    Minor,
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
    pub update_category: Option<UpdateCategory>,
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

/// Represents a software repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub source: PackageSource,
    pub enabled: bool,
    pub url: Option<String>,
    pub description: Option<String>,
}

impl Repository {
    pub fn new(name: String, source: PackageSource, enabled: bool, url: Option<String>) -> Self {
        Self {
            name,
            source,
            enabled,
            url,
            description: None,
        }
    }
}
