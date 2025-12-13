use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EnabledSources {
    pub apt: bool,
    pub flatpak: bool,
    pub snap: bool,
    pub npm: bool,
    pub pip: bool,
    pub deb: bool,
    pub appimage: bool,
}

impl Default for EnabledSources {
    fn default() -> Self {
        Self {
            apt: true,
            flatpak: true,
            snap: true,
            npm: true,
            pip: true,
            deb: true,
            appimage: true,
        }
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
