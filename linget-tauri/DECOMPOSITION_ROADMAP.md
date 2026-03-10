# LinGet Tauri Decomposition Roadmap

## Quick Reference: IPC Surface Summary

### Commands (Frontend → Backend)
**Total: 15 commands, 8 actively used, 2 dead code**

| Category | Commands | Active? | Notes |
|----------|----------|---------|-------|
| **Query** | `list_sources`, `list_installed_packages`, `list_available_packages`, `check_updates`, `search_packages` | ✅ Yes | Core listing operations |
| **Mutate** | `install_package`, `remove_package`, `update_package`, `update_all_packages` | ✅ Yes | Emit progress events |
| **Control** | `cancel_operation`, `get_operation_status` | ❌ Partial | cancel used, status not invoked |
| **Config** | `load_settings`, `save_settings`, `get_backend_sources` | ✅ Yes | Settings lifecycle |
| **Dead** | `get_package_info` | ❌ No | Stub only, never called |

### Events (Backend → Frontend)
**Total: 6 events, 3 actively used, 3 dead code**

| Event | Payload | Used? | Status |
|-------|---------|-------|--------|
| `operation-progress` | OperationProgressPayload | ✅ | Core tracking |
| `batch-update-progress` | BatchUpdateProgressPayload | ✅ | Bulk updates |
| `batch-update-completed` | — | ✅ | Bulk cleanup |
| `operation-log` | OperationLogPayload | ❌ | Defined, never emitted |
| `update-progress` | {name,source,status,error} | ❌ | Emitted, not listened |
| `batch-update-started` | {total,message} | ❌ | Emitted, not listened |

---

## Proposed Services Architecture

### 1. **Package Service** (Handler → Logic extraction)
```rust
// src-tauri/src/services/package_service.rs

pub struct PackageService;

impl PackageService {
    pub async fn list_installed(
        state: &AppState,
    ) -> Result<Vec<serde_json::Value>, String> {
        let manager = state.package_manager.lock().await;
        let packages = manager.list_all_installed().await?;
        Ok(packages.into_iter().map(package_to_json).collect())
    }

    pub async fn search(
        query: &str,
        state: &AppState,
    ) -> Result<Vec<serde_json::Value>, String> {
        let manager = state.package_manager.lock().await;
        let results = manager.search(query).await?;
        Ok(results.into_iter().map(package_to_json).collect())
    }

    pub async fn install(
        name: &str,
        source: &str,
        state: &AppState,
        window: &tauri::Window,
    ) -> Result<(), String> {
        // ... existing logic
        // OperationService::emit_progress(window, ...)
    }
    
    // remove(), update() similar patterns
}
```

**Stability**: ✅ Command handlers become thin routing → stable IPC

---

### 2. **Operation Service** (Event emission abstraction)
```rust
// src-tauri/src/services/operation_service.rs

pub struct OperationService;

impl OperationService {
    /// Emit operation progress (STABLE: 'operation-progress' event)
    pub fn emit_progress(
        window: &tauri::Window,
        operation_id: impl Into<String>,
        name: impl Into<String>,
        status: &str,
        progress: u64,
        message: impl Into<String>,
    ) -> tauri::Result<()> {
        window.emit(
            "operation-progress",
            serde_json::json!({
                "operation_id": operation_id.into(),
                "name": name.into(),
                "status": status,
                "progress": progress,
                "message": message.into(),
            }),
        )
    }

    pub fn emit_batch_progress(
        window: &tauri::Window,
        completed: usize,
        total: usize,
    ) -> tauri::Result<()> {
        let progress = if total == 0 { 100 } else { (completed * 100) / total };
        window.emit(
            "batch-update-progress",
            serde_json::json!({
                "completed": completed,
                "total": total,
                "progress": progress,
            }),
        )
    }

    pub fn emit_batch_completed(window: &tauri::Window) -> tauri::Result<()> {
        window.emit("batch-update-completed", serde_json::json!({}))
    }

    /// (Not yet used, but available for future log streaming)
    pub fn emit_log(
        window: &tauri::Window,
        operation_id: impl Into<String>,
        name: impl Into<String>,
        line: impl Into<String>,
        is_error: bool,
    ) -> tauri::Result<()> {
        window.emit(
            "operation-log",
            serde_json::json!({
                "operation_id": operation_id.into(),
                "name": name.into(),
                "line": line.into(),
                "is_error": is_error,
            }),
        )
    }
}
```

**Stability**: ✅ Event names centralized, easy to maintain contracts

---

### 3. **Settings Service** (Persistence & config)
```rust
// src-tauri/src/services/settings_service.rs

pub struct SettingsService;

impl SettingsService {
    pub fn load() -> AppSettings {
        match AppSettings::path() {
            Some(path) if path.exists() => {
                std::fs::read_to_string(path)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default()
            }
            _ => AppSettings::default(),
        }
    }

    pub fn save(settings: &AppSettings) -> Result<(), String> {
        if let Some(path) = AppSettings::path() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let json = serde_json::to_string_pretty(settings)
                .map_err(|e| e.to_string())?;
            std::fs::write(path, json).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn apply_to_manager(
        manager: &mut PackageManager,
        settings: &AppSettings,
    ) {
        let configured = enabled_sources_from_settings(settings);
        if configured.is_empty() {
            manager.set_enabled_sources(manager.available_sources());
        } else {
            manager.set_enabled_sources(configured);
        }
    }
}
```

**Stability**: ✅ Settings JSON contract remains unchanged

---

### 4. **Source Service** (Source enumeration & filtering)
```rust
// src-tauri/src/services/source_service.rs

pub struct SourceService;

impl SourceService {
    pub fn list_all() -> Vec<serde_json::Value> {
        PackageSource::ALL
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
            .collect()
    }

    pub fn list_with_availability(
        manager: &PackageManager,
        settings: &AppSettings,
    ) -> Vec<serde_json::Value> {
        let available = manager.available_sources();
        PackageSource::ALL
            .iter()
            .map(|source| {
                let is_available = available.contains(source);
                serde_json::json!({
                    "id": source.to_string(),
                    "name": source.to_string(),
                    "description": source.description(),
                    "icon": source.icon_name(),
                    "available": is_available,
                    "enabled": is_available && source_enabled(settings, *source, is_available)
                })
            })
            .collect()
    }
}
```

**Stability**: ✅ Output JSON contract maintained

---

### 5. **Utilities Module** (Conversions & helpers)
```rust
// src-tauri/src/utils/conversion.rs

pub fn package_to_json(package: Package) -> serde_json::Value {
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

fn enabled_sources_from_settings(settings: &AppSettings) -> HashSet<PackageSource> {
    settings
        .enabled_sources
        .iter()
        .filter_map(|source| PackageSource::from_str(source))
        .collect()
}

fn source_enabled(
    settings: &AppSettings,
    source: PackageSource,
    available: bool,
) -> bool {
    let configured = enabled_sources_from_settings(settings);
    if configured.is_empty() {
        available
    } else {
        configured.contains(&source)
    }
}
```

**Stability**: ✅ JSON contract preserved

---

### 6. **Lean Handler Layer** (Routing only)
```rust
// src-tauri/src/handlers/mod.rs

pub mod package_commands;
pub mod operation_commands;
pub mod settings_commands;

// Exposes: [list_sources, list_installed_packages, ...]
```

```rust
// src-tauri/src/handlers/package_commands.rs

use crate::services::PackageService;

#[tauri::command]
pub async fn list_installed_packages(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    PackageService::list_installed(&state).await
}

#[tauri::command]
pub async fn search_packages(
    query: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    PackageService::search(&query, &state).await
}

#[tauri::command]
pub async fn install_package(
    name: String,
    source: String,
    state: State<'_, Arc<AppState>>,
    window: tauri::Window,
) -> Result<(), String> {
    PackageService::install(&name, &source, &state, &window).await
}

// ... etc
```

**Stability**: ✅ Command names & signatures unchanged

---

## File Structure After Decomposition

```
src-tauri/src/
├── main.rs                      # ~50 lines (just setup)
│   ├── fn main() -> Result<()>
│   ├── create AppState
│   ├── register handlers
│   └── build & run tauri
├── state.rs                     # ~70 lines
│   ├── struct AppState
│   ├── struct AppSettings
│   ├── struct OperationState
│   └── impl AppState
├── handlers/                    # ~200 lines total
│   ├── mod.rs
│   ├── package_commands.rs      # list_*, search, install, remove, update
│   ├── operation_commands.rs    # cancel, get_status
│   └── settings_commands.rs     # load, save, list_sources, get_backend_sources
├── services/                    # ~400 lines total
│   ├── mod.rs
│   ├── package_service.rs       # Core package operations
│   ├── operation_service.rs     # Event emission
│   ├── settings_service.rs      # Persistence
│   └── source_service.rs        # Source enumeration
└── utils/                       # ~100 lines total
    ├── mod.rs
    ├── conversion.rs            # package_to_json, etc.
    └── error.rs                 # Common error types
```

**Total: 685 lines → same lines, better organized**

---

## Migration Steps (Minimal-Breaking)

### Phase 1: Extract Utilities (No IPC changes)
- [ ] Create `utils/conversion.rs` with `package_to_json()`
- [ ] Create `utils/error.rs` with common result types
- [ ] Update `main.rs` to use utilities
- **Breaking**: None

### Phase 2: Extract Services (Internal only)
- [ ] Create `services/settings_service.rs`
- [ ] Create `services/source_service.rs`
- [ ] Create `services/operation_service.rs` (abstraction layer)
- [ ] Create `services/package_service.rs` (logic extracted)
- [ ] Update `main.rs` handlers to call services
- **Breaking**: None (event names/command signatures unchanged)

### Phase 3: Create Handler Layer (Thin routing)
- [ ] Create `handlers/mod.rs`
- [ ] Create `handlers/settings_commands.rs`
- [ ] Create `handlers/operation_commands.rs`
- [ ] Create `handlers/package_commands.rs`
- [ ] Move command definitions (handlers become thin)
- **Breaking**: None (command names stay same)

### Phase 4: Clean Up Dead Code (Safe removal)
- [ ] Remove `get_package_info` command (frontend never calls)
- [ ] Remove `get_operation_status` command (frontend never calls)
- [ ] Remove dead event emissions (`update-progress`, `batch-update-started`)
- [ ] Remove `OperationLogPayload` listener setup in frontend
- **Breaking**: Minor (but unused APIs, safe to remove)

---

## Stability Checklist for Reviewers

### Before Merging Any Refactor:
- [ ] All 8 active commands still callable with same names & params
- [ ] Event names remain: `operation-progress`, `batch-update-progress`, `batch-update-completed`
- [ ] Package JSON schema unchanged: `name`, `version`, `source`, `status`, etc.
- [ ] SettingsData JSON schema unchanged: `dark_mode`, `auto_refresh`, `enabled_sources`
- [ ] OperationProgressPayload structure preserved: `operation_id`, `name`, `status`, `progress`, `message`
- [ ] Return type serialization matches (test with `serde_json::to_string()`)

### Integration Tests (Minimal Required):
```rust
#[tokio::test]
async fn test_list_installed_packages_returns_vec() {
    let state = test_app_state();
    let result = handlers::list_installed_packages(State::from(&state)).await;
    assert!(result.is_ok());
    let packages = result.unwrap();
    assert!(!packages.is_empty());
    // Verify JSON shape
    assert!(packages[0].get("name").is_some());
    assert!(packages[0].get("status").is_some());
}

#[tokio::test]
async fn test_operation_progress_emits() {
    // Mock window, verify emit called with correct event name
    let window = MockWindow::new();
    PackageService::install("vim", "APT", &state, &window).await.ok();
    assert_eq!(window.emitted_events, vec!["operation-progress"]);
}
```

---

## Frontend Impact (None required)
- Frontend code remains unchanged
- No new commands to invoke
- No new events to listen
- Same types/interfaces continue to work

---

## DX 9.6 Context
This decomposition enables:
1. **Testability**: Services can be unit tested independently
2. **Reusability**: Services can be used by multiple command handlers
3. **Maintainability**: Clear separation of concerns
4. **Extensibility**: Easy to add new commands without modifying main.rs
5. **Stability**: IPC contract centralized in service layer

All without breaking frontend or IPC surface.
