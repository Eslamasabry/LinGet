#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{Context, Result};
use dirs::config_dir;
use linget_backend_core::{Package, PackageSource, PackageStatus};
use linget_backends::backends::PackageManager;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{Emitter, State};
use tokio::sync::{Mutex, RwLock};

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

struct OperationState {
    cancelled: AtomicBool,
    progress: AtomicU64,
    message: Mutex<String>,
}

impl OperationState {
    fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            progress: AtomicU64::new(0),
            message: Mutex::new(String::new()),
        }
    }
}

struct AppState {
    package_manager: RwLock<PackageManager>,
    settings: Mutex<AppSettings>,
    operation_states: Mutex<HashMap<String, Arc<OperationState>>>,
}

fn enabled_sources_from_settings(settings: &AppSettings) -> HashSet<PackageSource> {
    settings
        .enabled_sources
        .iter()
        .filter_map(|source| PackageSource::from_str(source))
        .collect()
}

fn apply_settings_to_manager(manager: &mut PackageManager, settings: &AppSettings) {
    let configured_sources = enabled_sources_from_settings(settings);
    if configured_sources.is_empty() {
        manager.set_enabled_sources(manager.available_sources());
    } else {
        manager.set_enabled_sources(configured_sources);
    }
}

fn source_enabled(settings: &AppSettings, source: PackageSource, available: bool) -> bool {
    let configured_sources = enabled_sources_from_settings(settings);
    if configured_sources.is_empty() {
        available
    } else {
        configured_sources.contains(&source)
    }
}

fn package_status_value(status: PackageStatus) -> &'static str {
    match status {
        PackageStatus::Installed => "installed",
        PackageStatus::UpdateAvailable => "update_available",
        PackageStatus::NotInstalled => "not_installed",
        PackageStatus::Installing => "installing",
        PackageStatus::Removing => "removing",
        PackageStatus::Updating => "updating",
    }
}

fn package_to_json(package: Package) -> serde_json::Value {
    serde_json::json!({
        "name": package.name,
        "version": package.version,
        "available_version": package.available_version,
        "description": package.description,
        "source": package.source.to_string(),
        "status": package_status_value(package.status),
        "size": package.size,
        "size_display": package.size_display(),
        "homepage": package.homepage,
        "license": package.license,
        "maintainer": package.maintainer,
        "dependencies": package.dependencies,
    })
}

impl AppState {
    fn new() -> Self {
        let settings = AppSettings::load();
        let mut package_manager = PackageManager::new();
        apply_settings_to_manager(&mut package_manager, &settings);

        Self {
            package_manager: RwLock::new(package_manager),
            settings: Mutex::new(settings),
            operation_states: Mutex::new(HashMap::new()),
        }
    }

    async fn get_operation_state(&self, operation_id: &str) -> Arc<OperationState> {
        let mut states = self.operation_states.lock().await;
        if let Some(state) = states.get(operation_id) {
            return state.clone();
        }
        let state = Arc::new(OperationState::new());
        states.insert(operation_id.to_string(), state.clone());
        state
    }
}

#[tauri::command]
async fn list_sources() -> Result<Vec<serde_json::Value>, String> {
    Ok(PackageSource::ALL
        .iter()
        .map(|source| {
            serde_json::json!({
                "id": source.to_string(),
                "name": source.to_string(),
                "description": source.description(),
                "icon": source.icon_name(),
                "enabled": true
            })
        })
        .collect())
}

#[tauri::command]
async fn list_installed_packages(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.package_manager.read().await;
    let packages = manager
        .list_all_installed()
        .await
        .map_err(|e| e.to_string())?;

    Ok(packages.into_iter().map(package_to_json).collect())
}

#[tauri::command]
async fn list_available_packages(
    _source: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.package_manager.read().await;
    let packages = manager
        .list_all_installed()
        .await
        .map_err(|e| e.to_string())?;

    Ok(packages
        .into_iter()
        .filter(|p| p.status == PackageStatus::NotInstalled)
        .map(package_to_json)
        .collect())
}

#[tauri::command]
async fn check_updates(state: State<'_, Arc<AppState>>) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.package_manager.read().await;
    let updates = manager
        .check_all_updates()
        .await
        .map_err(|e| e.to_string())?;

    Ok(updates.into_iter().map(package_to_json).collect())
}

#[tauri::command]
async fn install_package(
    name: String,
    source: String,
    state: State<'_, Arc<AppState>>,
    window: tauri::Window,
) -> Result<(), String> {
    let source_enum =
        PackageSource::from_str(&source).ok_or_else(|| format!("Unknown source: {}", source))?;

    let manager = state.package_manager.read().await;
    let operation_id = format!("install-{}", name);
    let op_state = state.get_operation_state(&operation_id).await;

    op_state.cancelled.store(false, Ordering::Relaxed);
    op_state.progress.store(0, Ordering::Relaxed);
    *op_state.message.lock().await = format!("Installing {}", name);

    let _ = window.emit(
        "operation-progress",
        serde_json::json!({
            "operation_id": operation_id,
            "name": name,
            "status": "started",
            "progress": 0,
            "message": format!("Starting installation of {}", name),
        }),
    );

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

    let result = manager.install(&package).await;
    op_state.progress.store(100, Ordering::Relaxed);
    *op_state.message.lock().await = if result.is_ok() {
        format!("Installed {}", name)
    } else {
        format!(
            "Failed to install {}: {}",
            name,
            result.as_ref().err().expect("install error should exist")
        )
    };

    let _ = window.emit(
        "operation-progress",
        serde_json::json!({
            "operation_id": operation_id,
            "name": name,
            "status": if result.is_ok() { "completed" } else { "failed" },
            "progress": 100,
            "message": if result.is_ok() {
                format!("Installed {}", name)
            } else {
                format!("Failed to install {}: {}", name, result.as_ref().err().unwrap())
            },
        }),
    );

    result.map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_package(
    name: String,
    source: String,
    state: State<'_, Arc<AppState>>,
    window: tauri::Window,
) -> Result<(), String> {
    let source_enum =
        PackageSource::from_str(&source).ok_or_else(|| format!("Unknown source: {}", source))?;

    let manager = state.package_manager.read().await;
    let operation_id = format!("remove-{}", name);
    let op_state = state.get_operation_state(&operation_id).await;

    op_state.cancelled.store(false, Ordering::Relaxed);
    op_state.progress.store(0, Ordering::Relaxed);
    *op_state.message.lock().await = format!("Removing {}", name);

    let _ = window.emit(
        "operation-progress",
        serde_json::json!({
            "operation_id": operation_id,
            "name": name,
            "status": "started",
            "progress": 0,
            "message": format!("Starting removal of {}", name),
        }),
    );

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

    let result = manager.remove(&package).await;
    op_state.progress.store(100, Ordering::Relaxed);
    *op_state.message.lock().await = if result.is_ok() {
        format!("Removed {}", name)
    } else {
        format!(
            "Failed to remove {}: {}",
            name,
            result.as_ref().err().expect("remove error should exist")
        )
    };

    let _ = window.emit(
        "operation-progress",
        serde_json::json!({
            "operation_id": operation_id,
            "name": name,
            "status": if result.is_ok() { "completed" } else { "failed" },
            "progress": 100,
            "message": if result.is_ok() {
                format!("Removed {}", name)
            } else {
                format!("Failed to remove {}: {}", name, result.as_ref().err().unwrap())
            },
        }),
    );

    result.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_package(
    name: String,
    source: String,
    state: State<'_, Arc<AppState>>,
    window: tauri::Window,
) -> Result<(), String> {
    let source_enum =
        PackageSource::from_str(&source).ok_or_else(|| format!("Unknown source: {}", source))?;

    let manager = state.package_manager.read().await;
    let operation_id = format!("update-{}", name);
    let op_state = state.get_operation_state(&operation_id).await;

    op_state.cancelled.store(false, Ordering::Relaxed);
    op_state.progress.store(0, Ordering::Relaxed);
    *op_state.message.lock().await = format!("Updating {}", name);

    let _ = window.emit(
        "operation-progress",
        serde_json::json!({
            "operation_id": operation_id,
            "name": name,
            "status": "started",
            "progress": 0,
            "message": format!("Starting update of {}", name),
        }),
    );

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

    let result = manager.update(&package).await;
    op_state.progress.store(100, Ordering::Relaxed);
    *op_state.message.lock().await = if result.is_ok() {
        format!("Updated {}", name)
    } else {
        format!(
            "Failed to update {}: {}",
            name,
            result.as_ref().err().expect("update error should exist")
        )
    };

    let _ = window.emit(
        "operation-progress",
        serde_json::json!({
            "operation_id": operation_id,
            "name": name,
            "status": if result.is_ok() { "completed" } else { "failed" },
            "progress": 100,
            "message": if result.is_ok() {
                format!("Updated {}", name)
            } else {
                format!("Failed to update {}: {}", name, result.as_ref().err().unwrap())
            },
        }),
    );

    result.map_err(|e| e.to_string())
}

#[tauri::command]
async fn search_packages(
    query: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.package_manager.read().await;
    let results = manager.search(&query).await.map_err(|e| e.to_string())?;

    Ok(results.into_iter().map(package_to_json).collect())
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
async fn load_settings(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
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
    state: State<'_, Arc<AppState>>,
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

    let updated_settings = app_settings.clone();
    app_settings.save().map_err(|e| e.to_string())?;
    drop(app_settings);

    let mut manager = state.package_manager.write().await;
    apply_settings_to_manager(&mut manager, &updated_settings);

    Ok(())
}

#[tauri::command]
async fn get_backend_sources(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.package_manager.read().await;
    let available_sources = manager.available_sources();
    drop(manager);

    let settings = state.settings.lock().await;

    Ok(PackageSource::ALL
        .iter()
        .map(|source| {
            let available = available_sources.contains(source);
            serde_json::json!({
                "id": source.to_string(),
                "name": source.to_string(),
                "description": source.description(),
                "icon": source.icon_name(),
                "available": available,
                "enabled": available && source_enabled(&settings, *source, available)
            })
        })
        .collect())
}

#[tauri::command]
async fn update_all_packages(
    state: State<'_, Arc<AppState>>,
    window: tauri::Window,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.package_manager.read().await;
    let updates = manager
        .check_all_updates()
        .await
        .map_err(|e| e.to_string())?;

    let total = updates.len();
    let mut completed = 0;
    let mut results = Vec::new();

    let _ = window.emit(
        "batch-update-started",
        serde_json::json!({
            "total": total,
            "message": format!("Starting update of {} packages", total),
        }),
    );

    for pkg in updates {
        let name = pkg.name.clone();
        let source = pkg.source.to_string();
        let operation_id = format!("batch-update-{}-{}", source, name);

        let _ = window.emit(
            "operation-progress",
            serde_json::json!({
                "operation_id": operation_id,
                "name": name,
                "status": "started",
                "progress": 0,
                "message": format!("Starting update of {} from {}", name, source),
            }),
        );

        let result = manager.update(&pkg).await;
        let status = if result.is_ok() { "updated" } else { "failed" };
        let error = result.as_ref().err().map(|e| e.to_string());

        let _ = window.emit(
            "operation-progress",
            serde_json::json!({
                "operation_id": operation_id,
                "name": name,
                "status": if result.is_ok() { "completed" } else { "failed" },
                "progress": 100,
                "message": if result.is_ok() {
                    format!("Updated {} from {}", name, source)
                } else {
                    format!(
                        "Failed to update {} from {}: {}",
                        name,
                        source,
                        result.as_ref().err().unwrap()
                    )
                },
            }),
        );

        let _ = window.emit(
            "update-progress",
            serde_json::json!({
                "name": name,
                "source": source,
                "status": status,
                "error": error,
            }),
        );

        let json_result = serde_json::json!({
            "name": name,
            "source": source,
            "status": status,
            "error": error
        });
        results.push(json_result);
        completed += 1;

        let _ = window.emit(
            "batch-update-progress",
            serde_json::json!({
                "completed": completed,
                "total": total,
                "progress": if total == 0 { 100 } else { (completed * 100) / total },
            }),
        );
    }

    let success_count = results
        .iter()
        .filter(|r| r.get("status") == Some(&serde_json::json!("updated")))
        .count();
    let _ = window.emit(
        "batch-update-completed",
        serde_json::json!({
            "total": total,
            "completed": completed,
            "success": success_count,
            "failed": completed - success_count,
            "results": results,
        }),
    );

    Ok(results)
}

#[tauri::command]
async fn cancel_operation(
    operation_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let states = state.operation_states.lock().await;
    if let Some(op_state) = states.get(&operation_id) {
        op_state.cancelled.store(true, Ordering::Relaxed);
        *op_state.message.lock().await = "Cancelled by user".to_string();
    }
    Ok(())
}

#[tauri::command]
async fn get_operation_status(
    operation_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let states = state.operation_states.lock().await;
    if let Some(op_state) = states.get(&operation_id) {
        let is_cancelled = op_state.cancelled.load(Ordering::Relaxed);
        let progress = op_state.progress.load(Ordering::Relaxed);
        let message = op_state.message.lock().await.clone();

        Ok(serde_json::json!({
            "operation_id": operation_id,
            "is_cancelled": is_cancelled,
            "progress": progress,
            "message": message,
            "status": if is_cancelled { "cancelled" } else { "running" }
        }))
    } else {
        Ok(serde_json::json!({
            "operation_id": operation_id,
            "status": "unknown"
        }))
    }
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
