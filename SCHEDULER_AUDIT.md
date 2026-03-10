# LinGet DX9.5 Persistent Scheduler Audit Report

## Executive Summary

LinGet's scheduler system currently operates as an **in-process, UI-bound system** that requires the GTK GUI to remain open for scheduled tasks to execute. All scheduling logic resides in the GUI layer, with persistent state stored in TOML config files. There is **no systemd integration** or daemon mode for autonomous task execution.

---

## Current Scheduler Architecture

### 1. Core Data Model

**File:** `/home/eslam/Storage/Code/LinGet/src/models/scheduler.rs` (382 lines)

#### Key Types:

```rust
// Enum: Operations supported (Update/Install/Remove)
pub enum ScheduledOperation { Update, Install, Remove }

// Enum: Quick scheduling presets  
pub enum SchedulePreset {
    Tonight,           // 10 PM today or tomorrow
    TomorrowMorning,   // 8 AM
    TomorrowEvening,   // 8 PM
    InOneHour,
    InThreeHours,
    Custom,
}

// Core task definition
pub struct ScheduledTask {
    pub id: String,                          // UUID v4
    pub package_id: String,                  // "Source:Name" format
    pub package_name: String,
    pub source: PackageSource,               // Apt, Dnf, Pacman, etc.
    pub operation: ScheduledOperation,
    pub scheduled_at: DateTime<Utc>,         // Target execution time (UTC)
    pub created_at: DateTime<Utc>,
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,               // Failure reason if failed
}

// State container
pub struct SchedulerState {
    pub tasks: Vec<ScheduledTask>,
}
```

#### Key Methods:

| Method | Purpose | Constraints |
|--------|---------|-------------|
| `is_due()` | Check if task time <= now | Requires in-memory comparison |
| `is_pending()` | Check if task not completed | In-memory only |
| `due_tasks()` | Get all ready-to-run tasks | Returns Vec<&ScheduledTask> |
| `pending_tasks()` | Get all active (non-completed) tasks | Includes due + future tasks |
| `add_task()` | Add new task, replacing existing active ones per package | Max 1 active per package ID |
| `mark_completed()` | Mark task done, set completed_at | Idempotent |
| `mark_failed(error)` | Mark task failed, record error | Idempotent |
| `cleanup_old_tasks()` | Keep only last 50 completed tasks | Memory management |

**Serialization:** Uses `serde` + `toml` with default serde derives (lines 6-7, 126-127, 241-242).

---

### 2. Persistence Layer

**File:** `/home/eslam/Storage/Code/LinGet/src/models/config.rs`

#### Storage Structure:

```
Config location: ~/.config/linget/config.toml

[scheduler]
tasks = [
    # Each task serialized as:
    # { id = "...", package_id = "...", package_name = "...", 
    #   source = "...", operation = "...", scheduled_at = "...", 
    #   created_at = "...", completed = ..., completed_at = null,
    #   error = null }
]
```

#### Implementation:

```rust
// Line 116 in config.rs - part of Config struct
pub scheduler: SchedulerState,

// Lines 462-494: Load/Save cycle
impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir().join("linget").join("config.toml")
    }

    pub fn load() -> Self {
        // Read TOML, deserialize, return with defaults if missing
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => toml::from_str::<Config>(&content)
                Err(_) => Self::default()
            }
        } else { Self::default() }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(Self::config_path(), content)?;
        Ok(())
    }
}
```

**Persistence Guarantees:**
- ✅ Tasks saved to disk after each create/update (line 3914, 3935, 4046, 4064)
- ✅ Atomicity: Full config written as single file (no partial updates)
- ✅ Format: Human-readable TOML, versionable, debuggable
- ❌ No distributed locking (single-user only)
- ❌ No transaction log (point-in-time only)

---

### 3. Runtime Scheduling & Execution

**File:** `/home/eslam/Storage/Code/LinGet/src/ui/relm_app.rs` (5243 lines)

#### Initialization (Lines 1806-1812):

```rust
// During app startup (AppModel::view() method):
{
    let sender_scheduler = sender.clone();
    glib::timeout_add_local(std::time::Duration::from_secs(60), move || {
        sender_scheduler.input(AppMsg::CheckScheduledTasks);
        glib::ControlFlow::Continue
    });
    sender.input(AppMsg::CheckScheduledTasks);  // Immediate check on startup
}
```

**Timing:** 60-second check interval, repeating until app closes.

#### Core Message Flow:

```
1. USER: Schedules task via UI (schedule_popover.rs:104)
   └─> ScheduledTask created with future DateTime<Utc>
   └─> AppMsg::ScheduleTask sent

2. AppModel::update() → AppMsg::ScheduleTask (Line 3907)
   ├─ config.scheduler.add_task(task)
   ├─ config.save() [PERSISTS TO DISK]
   └─ sender.input(AppMsg::CheckScheduledTasks)

3. CheckScheduledTasks (Line 3963, runs every 60s)
   ├─ Guard: Skip if running_task_id.is_some() (exclusive execution)
   ├─ config.scheduler.due_tasks() [FILTERS: completed==false && now >= scheduled_at]
   ├─ Sort by scheduled_at (earliest first)
   └─ If any due: sender.input(AppMsg::ExecuteScheduledTask(id))

4. ExecuteScheduledTask (Line 3980)
   ├─ Fetch task from config.scheduler.tasks
   ├─ Set running_task_id
   ├─ glib::spawn_future_local {
   │  ├─ manager.list_all_installed()
   │  ├─ Find package by ID
   │  ├─ Call: manager.{update|install|remove}(pkg)
   │  └─ On result (Ok/Err):
   │     ├─ Update task.completed = true / task.mark_failed(err)
   │     ├─ config.scheduler.cleanup_old_tasks() [Keep 50]
   │     ├─ config.save() [PERSISTS COMPLETION STATE]
   │     └─ sender.input(AppMsg::ScheduledTaskCompleted{...})
   └─ }

5. ScheduledTaskCompleted (Line 4078) OR ScheduledTaskFailed (Line 4100)
   ├─ Clear running_task_id
   ├─ Notify user (toast + notification)
   └─ sender.input(AppMsg::CheckScheduledTasks) [Loop back to step 3]
```

#### Message Types (Lines 259-272):

```rust
pub enum AppMsg {
    ScheduleTask(crate::models::ScheduledTask),              // Create
    ScheduleBulkTasks(Vec<crate::models::ScheduledTask>),   // Bulk create
    CheckScheduledTasks,                                     // Poll due tasks (60s timer)
    ExecuteScheduledTask(String),                            // Run by task_id
    ScheduledTaskCompleted { task_id: String, ... },        // Success callback
    ScheduledTaskFailed { task_id: String, error: String }, // Failure callback
    CancelScheduledTask(String),                             // User cancellation
    TaskQueueAction(TaskQueueAction),                        // UI actions
}
```

---

### 4. Due Work Detection & Execution

**Key Assumption: App Must Be Running**

The scheduler **requires glib::timeout_add_local()** (Line 1808), which only functions while the GTK event loop is active:

```rust
// Line 3970-3976: Due work detection
let mut due_tasks: Vec<_> = config.scheduler.due_tasks();
due_tasks.sort_by_key(|task| task.scheduled_at);
due_tasks.first().map(|task| task.id.clone())

if let Some(task_id) = next_due {
    sender.input(AppMsg::ExecuteScheduledTask(task_id));
}
```

**Execution Method (Lines 4009-4074):**

```rust
glib::spawn_future_local(async move {
    // Non-blocking async execution via glib's event loop
    let packages = manager.list_all_installed().await?;
    let pkg = packages.iter().find(|p| p.id() == task.package_id);
    
    let result = match task.operation {
        Update => manager.update(pkg).await,
        Install => manager.install(pkg).await,
        Remove => manager.remove(pkg).await,
    };
    
    match result {
        Ok(_) => t.mark_completed(),
        Err(e) => t.mark_failed(e.to_string()),
    }
    config.save();  // Line 4046 or 4064
});
```

**Critical Constraints:**
1. ⚠️ **Single-threaded glib main loop**: Only one task executes at a time
2. ⚠️ **Event loop dependency**: Requires active GTK window
3. ⚠️ **60-second polling**: Precision ±60s (loose deadline)
4. ⚠️ **No persistent queue**: Tasks lost if app crashes before completion
5. ✅ **Async package operations**: Don't block UI during install/remove/update

---

## Task Lifecycle Visualization

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. SCHEDULED (user creates from UI)                             │
│    └─ saved to ~/.config/linget/config.toml                     │
│    └─ awaits scheduled_at >= Utc::now()                         │
├─────────────────────────────────────────────────────────────────┤
│ 2. DUE (every 60s check triggers)                               │
│    └─ if Utc::now() >= task.scheduled_at && !completed:        │
│       └─ AppMsg::ExecuteScheduledTask dispatched                │
├─────────────────────────────────────────────────────────────────┤
│ 3. RUNNING (exclusive lock)                                      │
│    └─ running_task_id = Some(id)                                │
│    └─ async operation started in glib event loop                │
│    └─ no concurrent tasks allowed                               │
├─────────────────────────────────────────────────────────────────┤
│ 4. COMPLETED or FAILED                                          │
│    ├─ completed = true, completed_at = Utc::now()              │
│    ├─ error = Some(msg) if failed                               │
│    ├─ config.save() (persists)                                  │
│    ├─ removed from loop after 50+ completed tasks               │
│    └─ displayed in task queue for manual retry/clear            │
└─────────────────────────────────────────────────────────────────┘
```

---

## UI Integration Points

**File:** `/home/eslam/Storage/Code/LinGet/src/ui/task_queue_view.rs`

- **Lines 62-69:** `retry_failed_task_ids()` - Filters for `error.is_some()`
- **Lines 118-121:** Task categorization (active/completed/failed)
- **Line 349:** `build_pending_task_row()` - Renders scheduled task display
- **Line 493:** `build_completed_task_row()` - Shows completion status

**File:** `/home/eslam/Storage/Code/LinGet/src/ui/widgets/schedule_popover.rs`

- **Lines 104-110:** Task creation from UI preset
- Serializes directly to `ScheduledTask::new(pkg_id, name, source, operation, scheduled_at)`

---

## Key Findings: Systemd Migration Seams

### What Would Need to Change:

#### 1. **Entry Point Gap**
- **Current:** App must stay running; glib timeout drives checks
- **Needed:** Standalone CLI command to execute queued tasks:
  ```rust
  // Hypothetical new command
  linget schedule exec [--task-id <id>] [--all-due]
  ```
  - Load config from `~/.config/linget/config.toml`
  - Filter due tasks
  - Execute without GTK loop
  - Exit with status code

#### 2. **Serialization Format is Ready**
- ✅ TOML is systemd-compatible (timers written in `.timer` and `.service` units)
- ✅ DateTime<Utc> serializes as ISO8601 strings (systemd-compatible)
- ✅ Example:
  ```toml
  scheduled_at = 2025-03-08T22:00:00Z  # Already in this format
  ```

#### 3. **Package Manager Access**
- **Current:** PackageManager inside Arc<Mutex<>>
- **Needed:** Refactor to allow CLI to load and use without GTK
  ```rust
  // Would need to extract PackageManager logic from GTK/relm4 coupling
  let manager = PackageManager::new().await;
  manager.update(&package).await?;
  ```

#### 4. **Notification System**
- **Current:** Uses GTK notifications (libadwaita)
- **Needed:** dbus-based notifications that work headless
  ```rust
  // Already uses:
  notifications::send_task_completed_notification(&package_name)
  // Should refactor to use zbus/dbus directly
  ```

#### 5. **Configuration Lock**
- **Current:** Single-user, no locking (GUI only)
- **Needed:** File lock when both GUI and systemd service run
  ```rust
  // Pseudo-code
  let _lock = LockFile::acquire(~/.config/linget/config.lock)?;
  let config = Config::load();
  // Update and save
  ```

---

## Systemd Integration Blueprint

### Proposed File Structure:
```
~/.config/systemd/user/linget-scheduled.service
~/.config/systemd/user/linget-scheduled.timer
```

### Service Unit:
```ini
[Unit]
Description=LinGet Scheduled Task Executor
After=network.target

[Service]
Type=oneshot
ExecStart=/usr/bin/linget schedule run
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
```

### Timer Unit:
```ini
[Unit]
Description=LinGet Scheduled Task Checker
Requires=linget-scheduled.service

[Timer]
# Check every minute (more frequent than current 60s)
OnBootSec=1min
OnUnitActiveSec=1min

[Install]
WantedBy=timers.target
```

### Required Code Changes:
1. **Extract CLI executor:**
   ```rust
   // src/cli/commands/schedule.rs (NEW)
   pub async fn run_scheduled() -> Result<()> {
       let config = Config::load();
       let pm = PackageManager::new().await?;
       
       for task in config.scheduler.due_tasks() {
           execute_task(&pm, task).await?;
       }
   }
   ```

2. **Decouple PackageManager from UI:**
   - Move from `backend/` crate-level into shareable module
   - Remove GTK dependencies from core package manager

3. **Add file locking:**
   ```rust
   use fs2::FileExt;
   let lock = File::create(lock_path)?;
   lock.try_lock_exclusive()?;
   let config = Config::load();
   // ... modify ...
   config.save()?;
   drop(lock);
   ```

4. **Decouple notifications:**
   - Use `zbus` (D-Bus) directly instead of GTK notifications
   - Works headless and with active GUI simultaneously

---

## Assumptions That Fail Without App Running

| Assumption | Impact | Workaround |
|-----------|--------|-----------|
| glib::timeout_add_local runs every 60s | Tasks delayed if app closes | systemd timer (1min accurate) |
| GTK event loop processes async tasks | Hangs if not running | tokio runtime + systemd service |
| Single-threaded execution (running_task_id guard) | No concurrent tasks | Built into service design (Type=oneshot) |
| User in same session as GTK | Notifications work | dbus works across sessions |
| Config not edited externally | No conflicts | File lock + atomic save |

---

## Summary: Code Seams for Systemd Migration

### Export Points:
- ✅ `/home/eslam/Storage/Code/LinGet/src/models/scheduler.rs` - Data model (no GTK deps)
- ✅ `/home/eslam/Storage/Code/LinGet/src/models/config.rs` - Config I/O (TOML)
- ❌ `/home/eslam/Storage/Code/LinGet/src/ui/relm_app.rs:3980` - Execution locked in async GTK
- ❌ `/home/eslam/Storage/Code/LinGet/src/backend/` - PackageManager may have GTK couplings

### File Locks Needed:
```
~/.config/linget/config.lock  (10-30s hold during I/O)
```

### Serialization Format:
- Format: TOML (not JSON)
- DateTime: ISO8601 UTC (`2025-03-08T22:00:00Z`)
- Task ID: UUID v4

### Command Entrypoint:
```bash
linget schedule run [--task-id <uuid>] [--dry-run]
linget schedule list
linget schedule cancel <uuid>
```

---

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|-----------|
| Tasks execute twice if GUI + systemd run | HIGH | Implement file locking |
| systemd timer fires before app exits | MEDIUM | Add "completed" check in systemd service |
| Package state changes between creation and execution | MEDIUM | Re-verify package exists + status during exec |
| Notification daemon not running in headless | LOW | Gracefully fall back to syslog |

---

## Conclusion

**Current State:** LinGet scheduler is a **UI-bound, GTK-dependent polling system** with no daemon or systemd integration. All persistent storage is TOML-based and ready for external consumption.

**To Migrate to systemd:**
1. Extract `PackageManager` execution logic from GTK event loop → CLI module
2. Add file locking to prevent race conditions
3. Create systemd `.service` + `.timer` units (config in user home)
4. Decouple notifications to use D-Bus instead of GTK
5. Add `linget schedule run` CLI command

**Effort Estimate:** 2-4 weeks for robust implementation (including testing, locking, and UI updates to reflect systemd status).

