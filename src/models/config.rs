use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

use super::PackageSource;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Whether to check for updates on startup
    pub check_updates_on_startup: bool,

    /// Update check interval in hours (0 = disabled)
    pub update_check_interval: u32,

    /// Whether to show system notifications
    pub show_notifications: bool,

    /// Enabled package sources
    pub enabled_sources: EnabledSources,

    /// Whether to run in background (system tray)
    pub run_in_background: bool,

    /// Start minimized to tray
    pub start_minimized: bool,

    /// Window width
    pub window_width: i32,

    /// Window height
    pub window_height: i32,

    /// Whether window was maximized
    pub window_maximized: bool,

    /// List of ignored package IDs (format: "Source:Name")
    #[serde(default)]
    pub ignored_packages: Vec<String>,

    /// Compact list density (smaller rows)
    #[serde(default)]
    pub ui_compact: bool,

    /// Show app icons in lists
    #[serde(default = "default_ui_show_icons")]
    pub ui_show_icons: bool,

    /// Last selected source filter (persisted across sessions)
    #[serde(default)]
    pub last_source_filter: Option<String>,

    /// Favorited package IDs (format: "Source:Name")
    #[serde(default)]
    pub favorite_packages: Vec<String>,
}

fn default_ui_show_icons() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EnabledSources {
    pub apt: bool,
    pub dnf: bool,
    pub pacman: bool,
    pub zypper: bool,
    pub flatpak: bool,
    pub snap: bool,
    pub npm: bool,
    pub pip: bool,
    pub pipx: bool,
    pub cargo: bool,
    pub brew: bool,
    pub aur: bool,
    pub conda: bool,
    pub mamba: bool,
    pub dart: bool,
    pub deb: bool,
    pub appimage: bool,
}

impl Default for EnabledSources {
    fn default() -> Self {
        Self {
            apt: true,
            dnf: true,
            pacman: true,
            zypper: true,
            flatpak: true,
            snap: true,
            npm: true,
            pip: true,
            pipx: true,
            cargo: true,
            brew: true,
            aur: true,
            conda: true,
            mamba: true,
            dart: true,
            deb: true,
            appimage: true,
        }
    }
}

impl EnabledSources {
    pub fn to_sources(&self) -> HashSet<PackageSource> {
        let mut sources = HashSet::new();
        if self.apt {
            sources.insert(PackageSource::Apt);
        }
        if self.dnf {
            sources.insert(PackageSource::Dnf);
        }
        if self.pacman {
            sources.insert(PackageSource::Pacman);
        }
        if self.zypper {
            sources.insert(PackageSource::Zypper);
        }
        if self.flatpak {
            sources.insert(PackageSource::Flatpak);
        }
        if self.snap {
            sources.insert(PackageSource::Snap);
        }
        if self.npm {
            sources.insert(PackageSource::Npm);
        }
        if self.pip {
            sources.insert(PackageSource::Pip);
        }
        if self.pipx {
            sources.insert(PackageSource::Pipx);
        }
        if self.cargo {
            sources.insert(PackageSource::Cargo);
        }
        if self.brew {
            sources.insert(PackageSource::Brew);
        }
        if self.aur {
            sources.insert(PackageSource::Aur);
        }
        if self.conda {
            sources.insert(PackageSource::Conda);
        }
        if self.mamba {
            sources.insert(PackageSource::Mamba);
        }
        if self.dart {
            sources.insert(PackageSource::Dart);
        }
        if self.deb {
            sources.insert(PackageSource::Deb);
        }
        if self.appimage {
            sources.insert(PackageSource::AppImage);
        }
        sources
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            check_updates_on_startup: true,
            update_check_interval: 24,
            show_notifications: true,
            enabled_sources: EnabledSources::default(),
            run_in_background: false,
            start_minimized: false,
            window_width: 1000,
            window_height: 700,
            window_maximized: false,
            ignored_packages: Vec::new(),
            ui_compact: false,
            ui_show_icons: default_ui_show_icons(),
            last_source_filter: None,
            favorite_packages: Vec::new(),
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("linget")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(config) => return config,
                    Err(e) => tracing::warn!("Failed to parse config: {}", e),
                },
                Err(e) => tracing::warn!("Failed to read config: {}", e),
            }
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)?;

        let content = toml::to_string_pretty(self)?;
        std::fs::write(Self::config_path(), content)?;
        Ok(())
    }
}
