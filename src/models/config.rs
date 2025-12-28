use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::appearance::AppearanceConfig;
use super::scheduler::SchedulerState;
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

    #[serde(default)]
    pub layout_mode: LayoutMode,

    /// Last selected source filter (persisted across sessions)
    #[serde(default)]
    pub last_source_filter: Option<String>,

    /// Favorited package IDs (format: "Source:Name")
    #[serde(default)]
    pub favorite_packages: Vec<String>,

    #[serde(default)]
    pub collections: HashMap<String, Vec<String>>,

    /// Whether onboarding has been completed
    #[serde(default)]
    pub onboarding_completed: bool,

    /// Recent search queries (last 5)
    #[serde(default)]
    pub recent_searches: Vec<String>,

    /// Dismissed recommendation package names (user chose to ignore these suggestions)
    #[serde(default)]
    pub dismissed_recommendations: Vec<String>,

    /// Enable vim-style keyboard navigation (j/k, g+h/l/u/s, etc.)
    #[serde(default)]
    pub vim_mode: bool,

    #[serde(default)]
    pub color_scheme: ColorScheme,

    #[serde(default)]
    pub accent_color: AccentColor,

    #[serde(default)]
    pub appearance: AppearanceConfig,

    #[serde(default)]
    pub scheduler: SchedulerState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum LayoutMode {
    Grid,
    #[default]
    List,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ColorScheme {
    System,
    Light,
    Dark,
    #[default]
    OledDark,
}

impl ColorScheme {
    pub fn display_name(&self) -> &'static str {
        match self {
            ColorScheme::System => "System",
            ColorScheme::Light => "Light",
            ColorScheme::Dark => "Dark",
            ColorScheme::OledDark => "OLED Dark",
        }
    }

    pub fn all() -> &'static [ColorScheme] {
        &[
            ColorScheme::System,
            ColorScheme::Light,
            ColorScheme::Dark,
            ColorScheme::OledDark,
        ]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AccentColor {
    #[default]
    System,
    Blue,
    Teal,
    Green,
    Yellow,
    Orange,
    Red,
    Pink,
    Purple,
    Slate,
}

impl AccentColor {
    pub fn display_name(&self) -> &'static str {
        match self {
            AccentColor::System => "System Default",
            AccentColor::Blue => "Blue",
            AccentColor::Teal => "Teal",
            AccentColor::Green => "Green",
            AccentColor::Yellow => "Yellow",
            AccentColor::Orange => "Orange",
            AccentColor::Red => "Red",
            AccentColor::Pink => "Pink",
            AccentColor::Purple => "Purple",
            AccentColor::Slate => "Slate",
        }
    }

    pub fn css_color(&self) -> Option<&'static str> {
        match self {
            AccentColor::System => None,
            AccentColor::Blue => Some("#3584e4"),
            AccentColor::Teal => Some("#2190a4"),
            AccentColor::Green => Some("#3a944a"),
            AccentColor::Yellow => Some("#c88800"),
            AccentColor::Orange => Some("#e66100"),
            AccentColor::Red => Some("#e62222"),
            AccentColor::Pink => Some("#d56199"),
            AccentColor::Purple => Some("#9141ac"),
            AccentColor::Slate => Some("#6e7a8a"),
        }
    }

    pub fn all() -> &'static [AccentColor] {
        &[
            AccentColor::System,
            AccentColor::Blue,
            AccentColor::Teal,
            AccentColor::Green,
            AccentColor::Yellow,
            AccentColor::Orange,
            AccentColor::Red,
            AccentColor::Pink,
            AccentColor::Purple,
            AccentColor::Slate,
        ]
    }
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
    pub fn set(&mut self, source: PackageSource, enabled: bool) {
        match source {
            PackageSource::Apt => self.apt = enabled,
            PackageSource::Dnf => self.dnf = enabled,
            PackageSource::Pacman => self.pacman = enabled,
            PackageSource::Zypper => self.zypper = enabled,
            PackageSource::Flatpak => self.flatpak = enabled,
            PackageSource::Snap => self.snap = enabled,
            PackageSource::Npm => self.npm = enabled,
            PackageSource::Pip => self.pip = enabled,
            PackageSource::Pipx => self.pipx = enabled,
            PackageSource::Cargo => self.cargo = enabled,
            PackageSource::Brew => self.brew = enabled,
            PackageSource::Aur => self.aur = enabled,
            PackageSource::Conda => self.conda = enabled,
            PackageSource::Mamba => self.mamba = enabled,
            PackageSource::Dart => self.dart = enabled,
            PackageSource::Deb => self.deb = enabled,
            PackageSource::AppImage => self.appimage = enabled,
        }
    }

    pub fn get(&self, source: PackageSource) -> bool {
        match source {
            PackageSource::Apt => self.apt,
            PackageSource::Dnf => self.dnf,
            PackageSource::Pacman => self.pacman,
            PackageSource::Zypper => self.zypper,
            PackageSource::Flatpak => self.flatpak,
            PackageSource::Snap => self.snap,
            PackageSource::Npm => self.npm,
            PackageSource::Pip => self.pip,
            PackageSource::Pipx => self.pipx,
            PackageSource::Cargo => self.cargo,
            PackageSource::Brew => self.brew,
            PackageSource::Aur => self.aur,
            PackageSource::Conda => self.conda,
            PackageSource::Mamba => self.mamba,
            PackageSource::Dart => self.dart,
            PackageSource::Deb => self.deb,
            PackageSource::AppImage => self.appimage,
        }
    }

    pub fn from_sources(sources: &HashSet<PackageSource>) -> Self {
        Self {
            apt: sources.contains(&PackageSource::Apt),
            dnf: sources.contains(&PackageSource::Dnf),
            pacman: sources.contains(&PackageSource::Pacman),
            zypper: sources.contains(&PackageSource::Zypper),
            flatpak: sources.contains(&PackageSource::Flatpak),
            snap: sources.contains(&PackageSource::Snap),
            npm: sources.contains(&PackageSource::Npm),
            pip: sources.contains(&PackageSource::Pip),
            pipx: sources.contains(&PackageSource::Pipx),
            cargo: sources.contains(&PackageSource::Cargo),
            brew: sources.contains(&PackageSource::Brew),
            aur: sources.contains(&PackageSource::Aur),
            conda: sources.contains(&PackageSource::Conda),
            mamba: sources.contains(&PackageSource::Mamba),
            dart: sources.contains(&PackageSource::Dart),
            deb: sources.contains(&PackageSource::Deb),
            appimage: sources.contains(&PackageSource::AppImage),
        }
    }

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
            layout_mode: LayoutMode::default(),
            last_source_filter: None,
            favorite_packages: Vec::new(),
            collections: HashMap::new(),
            onboarding_completed: false,
            recent_searches: Vec::new(),
            dismissed_recommendations: Vec::new(),
            vim_mode: false,
            color_scheme: ColorScheme::default(),
            accent_color: AccentColor::default(),
            appearance: AppearanceConfig::default(),
            scheduler: SchedulerState::default(),
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
        std::fs::create_dir_all(&dir).context("Failed to create config directory")?;

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(Self::config_path(), content).context("Failed to write config file")?;
        Ok(())
    }
}
