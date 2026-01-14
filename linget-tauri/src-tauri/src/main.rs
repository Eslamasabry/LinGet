#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{Context, Result};
use linget_backend_core::{Package, PackageSource, PackageStatus};
use linget_backends::backends::PackageManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize)]
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
}

impl AppState {
    fn new() -> Self {
        Self {
            package_manager: Mutex::new(PackageManager::new()),
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
    let source =
        PackageSource::from_str(&source).ok_or_else(|| format!("Unknown source: {}", source))?;

    let manager = state.package_manager.lock().await;
    let package = Package {
        name,
        version: String::new(),
        available_version: None,
        description: String::new(),
        source,
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
    let source =
        PackageSource::from_str(&source).ok_or_else(|| format!("Unknown source: {}", source))?;

    let manager = state.package_manager.lock().await;
    let package = Package {
        name,
        version: String::new(),
        available_version: None,
        description: String::new(),
        source,
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
    let source =
        PackageSource::from_str(&source).ok_or_else(|| format!("Unknown source: {}", source))?;

    let manager = state.package_manager.lock().await;
    let package = Package {
        name,
        version: String::new(),
        available_version: None,
        description: String::new(),
        source,
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
    let source = PackageSource::from_str(&source);
    if source.is_none() {
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
        "source": source.unwrap().to_string(),
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
async fn load_settings() -> Result<serde_json::Value, String> {
    let settings = AppSettings::default();
    Ok(serde_json::json!({
        "dark_mode": settings.dark_mode,
        "auto_refresh": settings.auto_refresh,
        "refresh_interval": settings.refresh_interval,
        "enabled_sources": settings.enabled_sources
    }))
}

#[tauri::command]
async fn save_settings(settings: serde_json::Value) -> Result<(), String> {
    // In a real app, save to config file
    // For now, just acknowledge
    Ok(())
}

#[tauri::command]
async fn get_backend_sources(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.package_manager.lock().await;
    let sources = manager.available_sources();

    Ok(sources
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.to_string(),
                "name": s.to_string(),
                "description": s.description(),
                "icon": s.icon_name(),
                "enabled": true
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
        results.push(serde_json::json!({
            "name": pkg.name,
            "source": pkg.source.to_string(),
            "status": "updating"
        }));
    }

    Ok(results)
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
        ])
        .run(tauri::generate_context!())
        .context("Failed to run Tauri application")
}
