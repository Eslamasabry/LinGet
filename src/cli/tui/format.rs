use super::theme::{
    badge_installed, badge_not_installed, badge_progress, badge_update, loading, muted, success,
    warning,
};
#[cfg(test)]
use crate::models::PackageSource;
use crate::models::{Package, PackageStatus};
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

pub fn format_package_version(package: &Package) -> String {
    match package.status {
        PackageStatus::UpdateAvailable | PackageStatus::Updating => {
            let available = package.available_version.as_deref().unwrap_or("?");
            format!("{}→{}", package.version, available)
        }
        PackageStatus::NotInstalled => package
            .available_version
            .clone()
            .unwrap_or_else(|| package.version.clone()),
        _ => package.version.clone(),
    }
}

pub fn package_status_short(status: PackageStatus) -> (&'static str, Style) {
    match status {
        PackageStatus::Installed => (" ✓ ", badge_installed()),
        PackageStatus::UpdateAvailable => (" ↑ ", badge_update()),
        PackageStatus::NotInstalled => (" ○ ", badge_not_installed()),
        PackageStatus::Installing | PackageStatus::Removing | PackageStatus::Updating => {
            (" ⟳ ", badge_progress())
        }
    }
}

pub fn package_status_label(status: PackageStatus) -> &'static str {
    match status {
        PackageStatus::Installed => "installed",
        PackageStatus::UpdateAvailable => "update available",
        PackageStatus::NotInstalled => "available",
        PackageStatus::Installing => "installing",
        PackageStatus::Removing => "removing",
        PackageStatus::Updating => "updating",
    }
}

pub fn package_status_style(status: PackageStatus) -> Style {
    match status {
        PackageStatus::Installed => success(),
        PackageStatus::UpdateAvailable => warning(),
        PackageStatus::NotInstalled => muted(),
        PackageStatus::Installing | PackageStatus::Removing | PackageStatus::Updating => loading(),
    }
}

pub fn truncate_to_width(text_value: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(text_value) <= max_width {
        return text_value.to_string();
    }
    if max_width == 1 {
        return "…".to_string();
    }

    let mut out = String::new();
    let mut width = 0usize;
    let target = max_width.saturating_sub(1);
    for ch in text_value.chars() {
        let char_width = UnicodeWidthStr::width(ch.to_string().as_str());
        if width + char_width > target {
            break;
        }
        out.push(ch);
        width += char_width;
    }
    out.push('…');
    out
}

pub fn truncate_middle_to_width(text_value: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(text_value) <= max_width {
        return text_value.to_string();
    }
    if max_width == 1 {
        return "…".to_string();
    }

    let target = max_width.saturating_sub(1);
    let left_target = target.div_ceil(2);
    let right_target = target / 2;

    let mut left = String::new();
    let mut left_width = 0usize;
    for ch in text_value.chars() {
        let char_width = UnicodeWidthStr::width(ch.to_string().as_str());
        if left_width + char_width > left_target {
            break;
        }
        left.push(ch);
        left_width += char_width;
    }

    let mut right = String::new();
    let mut right_width = 0usize;
    for ch in text_value.chars().rev() {
        let char_width = UnicodeWidthStr::width(ch.to_string().as_str());
        if right_width + char_width > right_target {
            break;
        }
        right.insert(0, ch);
        right_width += char_width;
    }

    format!("{left}…{right}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_package(status: PackageStatus) -> Package {
        Package {
            name: "demo".to_string(),
            version: "1.0.0".to_string(),
            available_version: Some("1.1.0".to_string()),
            description: "demo package".to_string(),
            source: PackageSource::Apt,
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
    fn truncate_middle_preserves_edges() {
        let truncated = truncate_middle_to_width("super-long-package-name", 12);
        assert_eq!(truncated, "super-…-name");
    }

    #[test]
    fn truncate_middle_handles_small_width() {
        assert_eq!(truncate_middle_to_width("abcdef", 1), "…");
        assert_eq!(truncate_middle_to_width("abcdef", 2), "a…");
    }

    #[test]
    fn format_package_version_prefers_available_version_for_updates() {
        assert_eq!(
            format_package_version(&make_package(PackageStatus::UpdateAvailable)),
            "1.0.0→1.1.0"
        );
    }

    #[test]
    fn package_status_copy_matches_detail_and_list_views() {
        assert_eq!(package_status_label(PackageStatus::Installed), "installed");
        assert_eq!(
            package_status_short(PackageStatus::UpdateAvailable)
                .0
                .trim(),
            "↑"
        );
        assert_eq!(package_status_label(PackageStatus::Removing), "removing");
    }
}
