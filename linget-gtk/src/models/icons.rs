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
            if name.contains('.') {
                return name.to_string();
            }
            source.icon_name().to_string()
        }
        PackageSource::Snap => {
            if let Some(icon) = try_find_snap_icon(name) {
                return icon;
            }
            source.icon_name().to_string()
        }
        _ => {
            if let Some(icon) = try_common_app_icon(name) {
                return icon;
            }
            source.icon_name().to_string()
        }
    }
}

fn try_find_snap_icon(name: &str) -> Option<String> {
    let snap_dir = Path::new("/var/lib/snapd/desktop/applications");
    if let Ok(entries) = std::fs::read_dir(snap_dir) {
        let prefix = format!("{}_", name);
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            if file_name_str.starts_with(&prefix) && file_name_str.ends_with(".desktop") {
                if let Some(icon) = parse_desktop_icon(&entry.path()) {
                    return Some(icon);
                }
            }
        }
    }
    None
}

fn try_common_app_icon(name: &str) -> Option<String> {
    let name_lower = name.to_lowercase();

    let common_icons = [
        ("firefox", "firefox"),
        ("chromium", "chromium"),
        ("chrome", "google-chrome"),
        ("vlc", "vlc"),
        ("gimp", "gimp"),
        ("inkscape", "inkscape"),
        ("blender", "blender"),
        ("libreoffice", "libreoffice-startcenter"),
        ("code", "visual-studio-code"),
        ("vscode", "visual-studio-code"),
        ("atom", "atom"),
        ("sublime", "sublime-text"),
        ("discord", "discord"),
        ("slack", "slack"),
        ("telegram", "telegram"),
        ("signal", "signal-desktop"),
        ("spotify", "spotify"),
        ("steam", "steam"),
        ("wine", "wine"),
        ("docker", "docker"),
        ("nodejs", "nodejs"),
        ("python", "python"),
        ("rust", "rust"),
        ("go", "golang"),
        ("git", "git"),
        ("vim", "vim"),
        ("neovim", "nvim"),
        ("emacs", "emacs"),
        ("htop", "htop"),
        ("obs", "com.obsproject.Studio"),
        ("kdenlive", "kdenlive"),
        ("audacity", "audacity"),
        ("krita", "krita"),
        ("thunderbird", "thunderbird"),
    ];

    for (pattern, icon) in common_icons {
        if name_lower.contains(pattern) {
            return Some(icon.to_string());
        }
    }

    None
}

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
