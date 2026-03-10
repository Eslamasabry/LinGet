# LinGet Tauri IPC Surface Analysis

## Architecture Overview
- **Frontend**: React SPA (TypeScript) @ `/ui/src/App.tsx` (1830 lines)
- **Backend**: Tauri Rust app @ `/src-tauri/src/main.rs` (685 lines)
- **IPC Protocol**: Tauri 2.x command/event system
- **Shared Backend Core**: External `linget-backend-core` + `linget-backends` crates

---

## STABLE IPC SURFACE (Must Remain Fixed During Decomposition)

### Backend-to-Frontend Events (Push/Streaming)
These are the ONLY events that the frontend subscribes to. Any new services/modules **MUST** emit through these channels:

| Event Name | Payload Type | Emitters | Consumers | Frequency |
|---|---|---|---|---|
| `operation-progress` | `OperationProgressPayload` | `install_package`, `remove_package`, `update_package` | Frontend (RunningOperation tracking) | Per operation start/completion |
| `operation-log` | `OperationLogPayload` | *Not currently emitted* | Frontend (RunningOperation log accumulation) | Per log line (ready for future use) |
| `batch-update-progress` | `BatchUpdateProgressPayload` | `update_all_packages` | Frontend (bulk operation progress bar) | Per package in batch |
| `batch-update-completed` | None | `update_all_packages` | Frontend (cleanup after bulk update) | Once per batch |
| `update-progress` | Not listened to | `update_all_packages` | None (emitted but unused) | Per package |
| `batch-update-started` | Not listened to | `update_all_packages` | None (emitted but unused) | Once |

**Key Observations:**
- `operation-log` is defined in frontend but **never emitted** by backend (dead code candidate)
- `update-progress`, `batch-update-started` are emitted but **never listened to** (dead code)
- Coupling point: Event payload structures are hardcoded in both UI and Rust

### Frontend-to-Backend Commands (RPC Calls)
These are the command handlers that must be kept stable. Decomposition should NOT change signatures:

| Command | Params | Returns | Backend Handler | Usage |
|---|---|---|---|---|
| `list_sources` | None | `Vec<SourceInfo>` | Direct source enumeration | On-demand |
| `list_installed_packages` | None | `Vec<Package>` | `manager.list_all_installed()` | Page load, refresh |
| `list_available_packages` | `source?: String` | `Vec<Package>` | Filter by `status=NotInstalled` | Browse tab |
| `check_updates` | None | `Vec<Package>` | `manager.check_all_updates()` | Updates tab load |
| `search_packages` | `query: String` | `Vec<Package>` | `manager.search(query)` | Browse search |
| `get_package_info` | `name: String, source: String` | `Package` | Fallback stub (returns empty) | *Not used* |
| `install_package` | `name: String, source: String` | `Result<(), String>` | `manager.install()` + emit progress | Install button |
| `remove_package` | `name: String, source: String` | `Result<(), String>` | `manager.remove()` + emit progress | Remove button |
| `update_package` | `name: String, source: String` | `Result<(), String>` | `manager.update()` + emit progress | Update button |
| `update_all_packages` | None | `Vec<{name,source,status,error}>` | Batch update with multi-emit | Bulk update |
| `cancel_operation` | `operation_id: String` | `Result<(), String>` | Set `operation_state.cancelled=true` | Cancel operation |
| `get_operation_status` | `operation_id: String` | `{operation_id,is_cancelled,progress,message,status}` | Read from `operation_states` map | *Not used* |
| `load_settings` | None | `SettingsData` | JSON file persistence | App init |
| `save_settings` | `settings: SettingsData` | `Result<(), String>` | JSON file write + apply to manager | Settings save |
| `get_backend_sources` | None | `Vec<SourceInfo>` | List with availability + enabled flags | Settings init |

---

## SHARED TYPE DEFINITIONS (IPC Contracts)

### Frontend Types (TypeScript)
```typescript
// Core domain
interface Package {
  name: string
  version: string
  available_version?: string
  description: string
  source: string  // String repr of source enum
  status: 'installed' | 'update_available' | 'not_installed' | 'installing' | 'removing' | 'updating'
  size?: number
  size_display: string
  homepage?: string
  license?: string
  maintainer?: string
  dependencies?: string[]
}

interface SourceInfo {
  id: string          // Source enum name (APT, Flatpak, Snap, npm, pip, pipx)
  name: string        // Display name
  icon: string        // Emoji/icon
  enabled: boolean    // User preference
  available?: boolean // System availability
  description?: string
  count?: number      // Not used currently
}

interface SettingsData {
  dark_mode: boolean
  auto_refresh: boolean
  refresh_interval: number
  enabled_sources: string[]  // Array of source IDs
}

// Event payloads (MUST STAY IN SYNC)
interface OperationProgressPayload {
  operation_id: string
  name: string
  status: 'started' | 'running' | 'completed' | 'failed'
  progress: number
  message: string
}

interface OperationLogPayload {
  operation_id: string
  name: string
  line: string
  is_error: boolean
}

interface BatchUpdateProgressPayload {
  completed: number
  total: number
  progress: number
}
```

### Backend Types (Rust - from linget-backend-core)
**Imported from external crates**, converted to JSON:
```rust
// From linget_backend_core
pub enum PackageSource { APT, Flatpak, Snap, npm, pip, pipx, ... }
pub enum PackageStatus { 
  Installed, UpdateAvailable, NotInstalled, 
  Installing, Removing, Updating 
}
pub struct Package {
  pub name: String,
  pub version: String,
  pub available_version: Option<String>,
  pub description: String,
  pub source: PackageSource,
  pub status: PackageStatus,
  pub size: Option<u64>,
  pub homepage: Option<String>,
  pub license: Option<String>,
  pub maintainer: Option<String>,
  pub dependencies: Vec<String>,
  pub install_date: Option<DateTime>,
  pub update_category: Option<String>,
}

// Local structs
#[derive(Serialize, Deserialize, Clone)]
struct AppSettings {
    dark_mode: bool,
    auto_refresh: bool,
    refresh_interval: u32,
    enabled_sources: Vec<String>,  // ["APT", "Flatpak", ...]
}

// Internal state tracking (NOT exposed on IPC)
struct OperationState {
    cancelled: AtomicBool,
    progress: AtomicU64,      // 0-100
    message: Mutex<String>,
}
```

---

## STATE MANAGEMENT PATTERNS

### Frontend Local State
- **Immediate UI**: React useState for tabs, modals, toasts, UI toggles
- **Package List**: Derived from invoke results, re-fetched on tab change
- **Running Operations**: Tracked via event listeners, keyed by operation_id
- **Settings**: Loaded on init, modified locally, saved via `save_settings`

### Backend Persistent State
- **AppState** (Mutex wrapped, shared):
  - `package_manager`: Core domain logic (from linget-backends)
  - `settings`: Loaded from `~/.config/linget/settings.json`
  - `operation_states`: Map of in-flight operations tracked by ID

### Current Operation Tracking
```
Frontend                          Backend
┌─────────────────────┐          ┌──────────────────────┐
│ RunningOperation[]   │          │ operation_states{}   │
│ - id (op-{pkg})     │◄──────────│ - cancelled flag     │
│ - progress          │  emit()   │ - progress (0-100)   │
│ - logs[]            │           │ - message            │
└─────────────────────┘          └──────────────────────┘
```

---

## DECOMPOSITION BOUNDARIES & HOOKS/SERVICES STRUCTURE

### Domain Decomposition Opportunities

#### 1. **Package Management Service** (STABLE BOUNDARY)
**Current**: Monolithic in `list_*`, `install/remove/update`, `search` commands

**Decomposition Target**:
```
src-tauri/src/
├── services/
│   ├── package_service.rs (STABLE IPC)
│   │   ├── list_installed()
│   │   ├── list_available()
│   │   ├── search()
│   │   ├── install()
│   │   └── update() / remove()
│   └── mod.rs
```
**Why stable**: All frontend commands map 1:1 to this service. Encapsulation doesn't break IPC.

#### 2. **Operation Tracking Service** (MUST REMAIN STABLE)
**Current**: Embedded in each command, hardcoded emits

**Decomposition Target**:
```
src-tauri/src/
├── services/
│   ├── operation_service.rs
│   │   ├── pub fn emit_progress()      // STABLE: event name "operation-progress"
│   │   ├── pub fn emit_log()           // STABLE: event name "operation-log"
│   │   ├── pub fn emit_batch_progress()// STABLE: event name "batch-update-progress"
│   │   ├── pub fn get_operation_state()
│   │   └── pub fn cancel_operation()
│   └── mod.rs
├── main.rs (just orchestrates)
```
**Stability Contract**:
- Event names + payload JSON schemas are PUBLIC API
- Internal `OperationState` structure is private
- Emission happens through stable service interface

#### 3. **Settings Service** (CLEAR BOUNDARY)
**Current**: Embedded in main.rs

**Decomposition Target**:
```
src-tauri/src/
├── services/
│   ├── settings_service.rs
│   │   ├── pub fn load() -> AppSettings
│   │   ├── pub fn save(settings: AppSettings) -> Result<()>
│   │   ├── pub fn apply_to_manager()
│   │   └── fn get_config_path() -> Option<PathBuf>
│   └── mod.rs
```
**Why clean**: Settings are isolated, only used in command handlers & manager initialization.

#### 4. **Source Management Service** (CLEAR BOUNDARY)
**Current**: Direct enum iteration in `list_sources`, `get_backend_sources`

**Decomposition Target**:
```
src-tauri/src/
├── services/
│   ├── source_service.rs
│   │   ├── pub fn list_all_sources() -> Vec<SourceInfo>
│   │   ├── pub fn get_available_sources(manager: &PackageManager) -> Vec<SourceInfo>
│   │   ├── pub fn apply_enabled_sources(manager: &mut PackageManager, enabled: &[String])
│   │   └── fn enabled_from_settings(settings: &AppSettings) -> HashSet<PackageSource>
│   └── mod.rs
```
**Why stable**: No direct IPC mutation, handlers remain thin.

---

## UNSTABLE / DEAD CODE (Candidates for Removal)

### Frontend
- **`OperationLogPayload` interface**: Defined but never received
- **`get_operation_status` command**: Implemented but never called from UI
- Reference to operation-log listener setup exists but backend never emits

### Backend
- **`update-progress` event**: Emitted in `update_all_packages` but never listened to
- **`batch-update-started` event**: Emitted but unused
- **`get_package_info` command**: Implemented but returns stub data (always empty struct)
- **`list_available_packages` with source param**: Ignores the `_source` parameter

---

## CRITICAL STABILITY GUARANTEES FOR REFACTORING

### MUST PRESERVE
1. **Event names** (hardcoded in frontend listener setup):
   - `"operation-progress"` → `OperationProgressPayload` shape
   - `"operation-log"` → `OperationLogPayload` shape (even if unused currently)
   - `"batch-update-progress"` → `BatchUpdateProgressPayload` shape
   - `"batch-update-completed"` → (no payload)

2. **Command handler names** (hardcoded in frontend invoke calls):
   - `search_packages`, `list_installed_packages`, `check_updates`, etc.
   - Parameter JSON keys: `{ query, name, source, settings, operation_id }`
   - Return type JSON serialization (matches `serde_json::json!()` current output)

3. **SettingsData contract**:
   - JSON keys: `dark_mode`, `auto_refresh`, `refresh_interval`, `enabled_sources`
   - Array of strings for sources (not enum serialization)

4. **Package type contract**:
   - JSON keys: `name`, `version`, `available_version`, `description`, `source`, `status`, `size`, `size_display`, `homepage`, `license`, `maintainer`, `dependencies`
   - Status values: strings `"installed"`, `"update_available"`, `"not_installed"`, `"installing"`, `"removing"`, `"updating"`
   - Source value: string enum representation (e.g., `"APT"`, `"Flatpak"`)

### CAN REFACTOR INTERNALLY
- Move `AppState` implementation details (doesn't cross IPC boundary)
- Extract `package_to_json()` conversion logic to dedicated module
- Reorganize operation state tracking (as long as map lookups by ID work the same)
- Decompose handler logic into service layer (as long as invocation points stay the same)

---

## HOOK/SERVICE MODULE RECOMMENDATION

### Suggested Directory Structure
```
src-tauri/src/
├── main.rs                   # Keep lean: just setup + routing
├── state.rs                  # AppState, shared types
├── handlers/                 # IPC command entry points (thin)
│   ├── mod.rs
│   ├── package_commands.rs   # invoke(list_*, search, install, remove, update)
│   ├── operation_commands.rs # invoke(cancel, get_status)
│   └── settings_commands.rs  # invoke(load, save, list_sources, get_backend_sources)
├── services/                 # Domain logic (reusable)
│   ├── mod.rs
│   ├── package_service.rs    # List, search, install/remove/update impl
│   ├── operation_service.rs  # Emit + track operations
│   ├── settings_service.rs   # Load/save + apply config
│   └── source_service.rs     # Source enumeration & filtering
└── utils/
    ├── mod.rs
    └── conversion.rs         # package_to_json, etc.
```

### Data Flow (Proposed)
```
Frontend invoke()
    ↓
handlers/package_commands.rs  (extract params, delegate)
    ↓
services/package_service.rs   (business logic, uses linget-backends)
    ↓
AppState (call package_manager, get_operation_state)
    ↓
services/operation_service.rs (emit events if needed)
    ↓
Return JSON result via window.emit() or Result type
```

---

## Integration Testing Implications

### IPC Contract Tests (Must Pass for Any Refactor)
```typescript
// Frontend test expectations (mock backend)
- invoke('search_packages', {query: 'python'}) 
  → Promise<Package[]> with correct fields
- listen('operation-progress') 
  → Event<{operation_id, name, status, progress, message}>
- invoke('install_package', {name, source}) 
  → Result<(), String>
```

```rust
// Rust integration tests
#[test]
async fn test_search_packages_returns_vec() { ... }
#[test]
async fn test_operation_progress_emits_on_install() { ... }
#[test]
async fn test_settings_persisted_across_restarts() { ... }
```

---

## DX 9.6 Notes
- Confirm this is context for **decomposing monolithic main.rs into hooks/services**
- Services should be **re-usable** (not tied to command handlers)
- IPC boundary is **PUBLIC API** — breaking changes require frontend updates
- Consider **async-channel** or **tokio::mpsc** for event emission if moving away from window.emit()

