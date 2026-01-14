#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{Context, Result};
use dirs::config_dir;
use linget_backend_core::{Package, PackageSource, PackageStatus};
use linget_backends::backends::PackageManager;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Clone)]
struct PackageJson {
    name: String,
    version: String,
    available_version: Option<String>,
    description: String,
    source: String,
    status: String,
    size: Option<u64>,
    size_display: String,
    homepage: Option<String>,
    license: Option<String>,
    maintainer: Option<String>,
    dependencies: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct AppSettings {
    dark_mode: bool,
    auto_refresh: bool,
    refresh_interval: u32,
    enabled_sources: Vec<String>,
}

impl AppSettings {
    fn path() -> Option<PathBuf> {
        config_dir().map(|mut p| {
            p.push("linget");
            p.push("settings.json");
            p
        })
    }

    fn load() -> Self {
        match Self::path() {
            Some(path) if path.exists() => std::fs::read_to_string(path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            _ => Self::default(),
        }
    }

    fn save(&self) -> Result<()> {
        if let Some(path) = Self::path() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let json = serde_json::to_string_pretty(self)?;
            std::fs::write(path, json)?;
        }
        Ok(())
    }
}

impl From<Package> for PackageJson {
    fn from(p: Package) -> Self {
        let size_display = p.size_display();
        Self {
            name: p.name,
            version: p.version,
            available_version: p.available_version,
            description: p.description,
            source: p.source.to_string(),
            status: match p.status {
                PackageStatus::Installed => "installed",
                PackageStatus::UpdateAvailable => "update_available",
                PackageStatus::NotInstalled => "not_installed",
                PackageStatus::Installing => "installing",
                PackageStatus::Removing => "removing",
                PackageStatus::Updating => "updating",
            }
            .to_string(),
            size: p.size,
            size_display,
            homepage: p.homepage,
            license: p.license,
            maintainer: p.maintainer,
            dependencies: p.dependencies,
        }
    }
}

struct AppState {
    package_manager: Mutex<PackageManager>,
    settings: Mutex<AppSettings>,
}

impl AppState {
    fn new() -> Self {
        Self {
            package_manager: Mutex::new(PackageManager::new()),
            settings: Mutex::new(AppSettings::load()),
        }
    }
}

#[tauri::command]
async fn list_sources() -> Result<Vec<serde_json::Value>, String> {
    Ok(PackageSource::ALL
        .iter()
        .map(|s| {
            serde_json::json!({
                "source": s.to_string(),
                "display": s.description(),
                "icon": s.icon_name(),
                "enabled": true
            })
        })
        .collect())
}

#[tauri::command]
async fn list_installed_packages(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.package_manager.lock().await;
    let packages = manager
        .list_all_installed()
        .await
        .map_err(|e| e.to_string())?;

    Ok(packages
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "version": p.version,
                "available_version": p.available_version,
                "description": p.description,
                "source": p.source.to_string(),
                "status": match p.status {
                    PackageStatus::Installed => "installed",
                    PackageStatus::UpdateAvailable => "update_available",
                    PackageStatus::NotInstalled => "not_installed",
                    _ => "unknown",
                },
                "size": p.size,
                "size_display": p.size_display(),
            })
        })
        .collect())
}

#[tauri::command]
async fn list_available_packages(
    _source: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.package_manager.lock().await;
    let packages = manager
        .list_all_installed()
        .await
        .map_err(|e| e.to_string())?;

    Ok(packages
        .into_iter()
        .filter(|p| p.status == PackageStatus::NotInstalled)
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "version": p.version,
                "available_version": p.available_version,
                "description": p.description,
                "source": p.source.to_string(),
                "status": "not_installed",
                "size": p.size,
                "size_display": p.size_display(),
            })
        })
        .collect())
}

#[tauri::command]
async fn check_updates(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.package_manager.lock().await;
    let updates = manager
        .check_all_updates()
        .await
        .map_err(|e| e.to_string())?;

    Ok(updates
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "version": p.version,
                "available_version": p.available_version,
                "description": p.description,
                "source": p.source.to_string(),
                "status": "update_available",
                "size": p.size,
                "size_display": p.size_display(),
            })
        })
        .collect())
}

#[tauri::command]
async fn install_package(
    name: String,
    source: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let source_enum =
        PackageSource::from_str(&source).ok_or_else(|| format!("Unknown source: {}", source))?;

    let manager = state.package_manager.lock().await;

    // Create a minimal package for the install operation
    let package = Package {
        name: name.clone(),
        version: String::new(),
        available_version: None,
        description: String::new(),
        source: source_enum,
        status: PackageStatus::Installing,
        size: None,
        homepage: None,
        license: None,
        maintainer: None,
        dependencies: Vec::new(),
        install_date: None,
        update_category: None,
    };

    manager.install(&package).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_package(
    name: String,
    source: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let source_enum =
        PackageSource::from_str(&source).ok_or_else(|| format!("Unknown source: {}", source))?;

    let manager = state.package_manager.lock().await;

    let package = Package {
        name: name.clone(),
        version: String::new(),
        available_version: None,
        description: String::new(),
        source: source_enum,
        status: PackageStatus::Removing,
        size: None,
        homepage: None,
        license: None,
        maintainer: None,
        dependencies: Vec::new(),
        install_date: None,
        update_category: None,
    };

    manager.remove(&package).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_package(
    name: String,
    source: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let source_enum =
        PackageSource::from_str(&source).ok_or_else(|| format!("Unknown source: {}", source))?;

    let manager = state.package_manager.lock().await;

    let package = Package {
        name: name.clone(),
        version: String::new(),
        available_version: None,
        description: String::new(),
        source: source_enum,
        status: PackageStatus::Updating,
        size: None,
        homepage: None,
        license: None,
        maintainer: None,
        dependencies: Vec::new(),
        install_date: None,
        update_category: None,
    };

    manager.update(&package).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn search_packages(
    query: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.package_manager.lock().await;
    let results = manager.search(&query).await.map_err(|e| e.to_string())?;

    Ok(results
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "version": p.version,
                "available_version": p.available_version,
                "description": p.description,
                "source": p.source.to_string(),
                "status": match p.status {
                    PackageStatus::Installed => "installed",
                    PackageStatus::UpdateAvailable => "update_available",
                    PackageStatus::NotInstalled => "not_installed",
                    _ => "unknown",
                },
                "size": p.size,
                "size_display": p.size_display(),
            })
        })
        .collect())
}

#[tauri::command]
async fn get_package_info(name: String, source: String) -> Result<serde_json::Value, String> {
    let source_enum = PackageSource::from_str(&source);
    if source_enum.is_none() {
        return Ok(serde_json::json!({
            "name": name,
            "source": source,
            "version": "",
            "description": "",
            "status": "unknown"
        }));
    }

    Ok(serde_json::json!({
        "name": name,
        "source": source,
        "version": "",
        "description": "",
        "status": "unknown",
        "homepage": null,
        "license": null,
        "maintainer": null,
        "dependencies": []
    }))
}

#[tauri::command]
async fn load_settings(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let settings = state.settings.lock().await;
    Ok(serde_json::json!({
        "dark_mode": settings.dark_mode,
        "auto_refresh": settings.auto_refresh,
        "refresh_interval": settings.refresh_interval,
        "enabled_sources": settings.enabled_sources
    }))
}

#[tauri::command]
async fn save_settings(
    settings: serde_json::Value,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let mut app_settings = state.settings.lock().await;

    if let Some(dark_mode) = settings.get("dark_mode").and_then(|v| v.as_bool()) {
        app_settings.dark_mode = dark_mode;
    }
    if let Some(auto_refresh) = settings.get("auto_refresh").and_then(|v| v.as_bool()) {
        app_settings.auto_refresh = auto_refresh;
    }
    if let Some(interval) = settings.get("refresh_interval").and_then(|v| v.as_u64()) {
        app_settings.refresh_interval = interval as u32;
    }
    if let Some(enabled) = settings.get("enabled_sources").and_then(|v| v.as_array()) {
        app_settings.enabled_sources = enabled
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }

    app_settings.save().map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_backend_sources(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.package_manager.lock().await;
    let sources = manager.available_sources();

    let settings = state.settings.lock().await;
    let enabled_sources: std::collections::HashSet<String> =
        settings.enabled_sources.iter().cloned().collect();

    Ok(sources
        .iter()
        .map(|s| {
            let id = s.to_string();
            let is_enabled = enabled_sources.is_empty() || enabled_sources.contains(&id);
            serde_json::json!({
                "id": id,
                "name": s.to_string(),
                "description": s.description(),
                "icon": s.icon_name(),
                "enabled": is_enabled
            })
        })
        .collect())
}

#[tauri::command]
async fn update_all_packages(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.package_manager.lock().await;
    let updates = manager
        .check_all_updates()
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for pkg in updates {
        let update_result = manager.update(&pkg).await;

        results.push(serde_json::json!({
            "name": pkg.name,
            "source": pkg.source.to_string(),
            "status": if update_result.is_ok() { "updated" } else { "failed" },
            "error": update_result.err().map(|e| e.to_string())
        }));
    }

    Ok(results)
}

#[tauri::command]
async fn cancel_operation(
    _name: String,
    _source: String,
    _state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // TODO: Implement operation cancellation
    // This would require adding cancellation tokens to the backend operations
    Ok(())
}

#[tauri::command]
async fn get_operation_status(
    _name: String,
    _source: String,
    _state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    // TODO: Implement real-time operation status
    Ok(serde_json::json!({
        "status": "idle",
        "progress": 0,
        "message": ""
    }))
}

fn main() -> Result<()> {
    let app_state = Arc::new(AppState::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            list_sources,
            list_installed_packages,
            list_available_packages,
            check_updates,
            install_package,
            remove_package,
            update_package,
            search_packages,
            get_package_info,
            load_settings,
            save_settings,
            get_backend_sources,
            update_all_packages,
            cancel_operation,
            get_operation_status,
        ])
        .run(tauri::generate_context!())
        .context("Failed to run Tauri application")
}
