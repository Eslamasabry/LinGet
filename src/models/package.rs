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
