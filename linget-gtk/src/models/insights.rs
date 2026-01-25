#![allow(dead_code)]

use crate::models::PackageSource;
use chrono::{DateTime, Local, NaiveDate};

#[derive(Debug, Clone, Default)]
pub struct PackageInsights {
    pub install_date: Option<DateTime<Local>>,
    pub dependencies_count: usize,
    pub reverse_dependencies: Vec<String>,
    pub config_paths: Vec<String>,
    pub log_command: Option<String>,
    pub is_safe_to_remove: bool,
    pub shared_deps_count: usize,
}

impl PackageInsights {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_install_date(mut self, date: Option<DateTime<Local>>) -> Self {
        self.install_date = date;
        self
    }

    pub fn with_dependencies(mut self, count: usize, shared: usize) -> Self {
        self.dependencies_count = count;
        self.shared_deps_count = shared;
        self
    }

    pub fn with_reverse_deps(mut self, deps: Vec<String>) -> Self {
        self.is_safe_to_remove = deps.is_empty();
        self.reverse_dependencies = deps;
        self
    }

    pub fn with_config_paths(mut self, paths: Vec<String>) -> Self {
        self.config_paths = paths;
        self
    }

    pub fn with_log_command(mut self, cmd: Option<String>) -> Self {
        self.log_command = cmd;
        self
    }

    pub fn install_age_display(&self) -> Option<String> {
        let install_date = self.install_date?;
        let now = Local::now();
        let duration = now.signed_duration_since(install_date);

        let days = duration.num_days();
        if days == 0 {
            Some("Today".to_string())
        } else if days == 1 {
            Some("Yesterday".to_string())
        } else if days < 7 {
            Some(format!("{} days ago", days))
        } else if days < 30 {
            let weeks = days / 7;
            Some(format!(
                "{} week{} ago",
                weeks,
                if weeks == 1 { "" } else { "s" }
            ))
        } else if days < 365 {
            let months = days / 30;
            Some(format!(
                "{} month{} ago",
                months,
                if months == 1 { "" } else { "s" }
            ))
        } else {
            let years = days / 365;
            Some(format!(
                "{} year{} ago",
                years,
                if years == 1 { "" } else { "s" }
            ))
        }
    }

    pub fn deps_display(&self) -> String {
        if self.dependencies_count == 0 {
            "No dependencies".to_string()
        } else if self.shared_deps_count > 0 {
            format!(
                "{} package{} (shared with {} other{})",
                self.dependencies_count,
                if self.dependencies_count == 1 {
                    ""
                } else {
                    "s"
                },
                self.shared_deps_count,
                if self.shared_deps_count == 1 { "" } else { "s" }
            )
        } else {
            format!(
                "{} package{}",
                self.dependencies_count,
                if self.dependencies_count == 1 {
                    ""
                } else {
                    "s"
                }
            )
        }
    }

    pub fn reverse_deps_display(&self) -> String {
        if self.reverse_dependencies.is_empty() {
            "Not required by any package".to_string()
        } else {
            format!(
                "Required by {} package{}",
                self.reverse_dependencies.len(),
                if self.reverse_dependencies.len() == 1 {
                    ""
                } else {
                    "s"
                }
            )
        }
    }

    pub fn safe_to_remove_display(&self) -> (&'static str, &'static str) {
        if self.is_safe_to_remove {
            ("emblem-ok-symbolic", "Safe to remove")
        } else {
            ("dialog-warning-symbolic", "May break other packages")
        }
    }
}

pub fn parse_install_date(date_str: &str) -> Option<DateTime<Local>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.with_timezone(&Local));
    }

    if let Ok(dt) = DateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S %z") {
        return Some(dt.with_timezone(&Local));
    }

    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        let dt = date.and_hms_opt(0, 0, 0)?;
        return Some(DateTime::from_naive_utc_and_offset(
            dt,
            *Local::now().offset(),
        ));
    }

    if let Ok(ts) = date_str.parse::<i64>() {
        return DateTime::from_timestamp(ts, 0).map(|dt| dt.with_timezone(&Local));
    }

    None
}

pub fn guess_config_paths(name: &str, source: PackageSource) -> Vec<String> {
    let home = dirs::home_dir().unwrap_or_default();
    let config_dir = dirs::config_dir().unwrap_or_default();
    let data_dir = dirs::data_dir().unwrap_or_default();

    let mut paths = Vec::new();

    let config_path = config_dir.join(name);
    if config_path.exists() {
        paths.push(config_path.to_string_lossy().to_string());
    }

    let dot_config = home.join(format!(".{}", name));
    if dot_config.exists() {
        paths.push(dot_config.to_string_lossy().to_string());
    }

    let dotfile = home.join(format!(".{}rc", name));
    if dotfile.exists() {
        paths.push(dotfile.to_string_lossy().to_string());
    }

    let data_path = data_dir.join(name);
    if data_path.exists() {
        paths.push(data_path.to_string_lossy().to_string());
    }

    if source == PackageSource::Flatpak {
        let flatpak_data = home.join(".var/app").join(name);
        if flatpak_data.exists() {
            paths.push(flatpak_data.to_string_lossy().to_string());
        }
    }

    if source == PackageSource::Snap {
        let snap_data = home.join("snap").join(name);
        if snap_data.exists() {
            paths.push(snap_data.to_string_lossy().to_string());
        }
    }

    paths
}

pub fn guess_log_command(name: &str, source: PackageSource) -> Option<String> {
    match source {
        PackageSource::Apt | PackageSource::Dnf | PackageSource::Pacman | PackageSource::Zypper => {
            Some(format!("journalctl -u {} --no-pager -n 50", name))
        }
        PackageSource::Flatpak => Some(format!("flatpak run --command=journalctl {} -n 50", name)),
        PackageSource::Snap => Some(format!("snap logs {}", name)),
        _ => None,
    }
}
