use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::models::PackageSource;

/// Cache for package icon names
static ICON_CACHE: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

/// Get the icon name for a package
pub fn get_package_icon(name: &str, source: PackageSource) -> String {
    // Check cache first
    let cache_key = format!("{}:{}", source, name);
    let name_lower = name.to_lowercase();
    let cache_key_lower = format!("{}:{}", source, name_lower);
    let last_segment_lower = name_lower
        .split('.')
        .next_back()
        .unwrap_or(&name_lower)
        .to_string();

    if let Ok(cache) = ICON_CACHE.read() {
        if let Some(ref map) = *cache {
            for key in [
                &cache_key,
                &cache_key_lower,
                &name_lower,
                &last_segment_lower,
            ] {
                if let Some(icon) = map.get(key) {
                    return icon.clone();
                }
            }
        }
    }

    // Try to find the icon
    let icon = find_icon(name, source);

    // Cache the result
    if let Ok(mut cache) = ICON_CACHE.write() {
        let map = cache.get_or_insert_with(HashMap::new);
        map.insert(cache_key, icon.clone());
    }

    icon
}

fn find_icon(name: &str, source: PackageSource) -> String {
    match source {
        PackageSource::Flatpak => {
            // Flatpak app IDs are usually the icon name
            // Try common patterns
            if name.contains('.') {
                // It's likely an app ID like "com.spotify.Client"
                return name.to_string();
            }
            source.icon_name().to_string()
        }
        PackageSource::Snap => {
            // Try to find snap icon from desktop files
            if let Some(icon) = find_snap_icon(name) {
                return icon;
            }
            source.icon_name().to_string()
        }
        PackageSource::Apt => {
            // Try to find from desktop files
            if let Some(icon) = find_desktop_icon(name) {
                return icon;
            }
            source.icon_name().to_string()
        }
        _ => source.icon_name().to_string(),
    }
}

/// Search for icon in snap desktop files
fn find_snap_icon(name: &str) -> Option<String> {
    let desktop_dirs = [
        PathBuf::from("/var/lib/snapd/desktop/applications"),
        dirs::data_dir()?.join("applications"),
    ];

    for dir in desktop_dirs {
        if !dir.exists() {
            continue;
        }

        // Try to find a desktop file matching the snap name
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    // Snap desktop files are usually named like "snapname_appname.desktop"
                    if filename.starts_with(&format!("{}_", name))
                        || filename.starts_with(&format!("{}.", name))
                    {
                        if let Some(icon) = parse_desktop_icon(&path) {
                            return Some(icon);
                        }
                    }
                }
            }
        }
    }

    None
}

/// Search for icon in system desktop files
fn find_desktop_icon(name: &str) -> Option<String> {
    let desktop_dirs = [
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        dirs::data_dir()?.join("applications"),
    ];

    for dir in desktop_dirs {
        if !dir.exists() {
            continue;
        }

        // Try exact match first
        let desktop_file = dir.join(format!("{}.desktop", name));
        if desktop_file.exists() {
            if let Some(icon) = parse_desktop_icon(&desktop_file) {
                return Some(icon);
            }
        }

        // Try with org. prefix (common for GNOME apps)
        let desktop_file = dir.join(format!("org.gnome.{}.desktop", capitalize(name)));
        if desktop_file.exists() {
            if let Some(icon) = parse_desktop_icon(&desktop_file) {
                return Some(icon);
            }
        }
    }

    None
}

/// Parse Icon= line from a desktop file
fn parse_desktop_icon(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;

    for line in content.lines() {
        if line.starts_with("Icon=") {
            let icon = line.trim_start_matches("Icon=").trim();
            if !icon.is_empty() {
                return Some(icon.to_string());
            }
        }
    }

    None
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

/// Initialize the icon cache by scanning desktop files
pub fn init_icon_cache() {
    // This runs in the background to pre-populate common icons
    std::thread::spawn(|| {
        let mut cache = HashMap::new();

        // Scan common desktop file locations
        let dirs = [
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/var/lib/snapd/desktop/applications"),
        ];

        for dir in dirs {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten().take(500) {
                    let path = entry.path();
                    if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                        if let (Some(stem), Some(icon)) = (
                            path.file_stem().and_then(|s| s.to_str()),
                            parse_desktop_icon(&path),
                        ) {
                            let stem_lower = stem.to_lowercase();
                            cache.insert(stem_lower.clone(), icon.clone());
                            if let Some(last) = stem_lower.split('.').next_back() {
                                cache.entry(last.to_string()).or_insert(icon);
                            }
                        }
                    }
                }
            }
        }

        if let Ok(mut global_cache) = ICON_CACHE.write() {
            *global_cache = Some(cache);
        }

        tracing::debug!("Icon cache initialized");
    });
}
