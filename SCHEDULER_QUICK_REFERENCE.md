# LinGet Scheduler - Quick Technical Reference

## File Locations & Function Map

### 1. Data Model & Serialization
- **File:** `src/models/scheduler.rs` (382 lines)
- **Task struct:** Line 127-139 (`ScheduledTask` with 9 fields)
- **State container:** Line 241-244 (`SchedulerState { tasks: Vec<ScheduledTask> }`)
- **Key functions:**
  - `ScheduledTask::new()` (Line 143-162): Creates UUID, sets created_at = Utc::now()
  - `ScheduledTask::is_due()` (Line 164-166): Returns `!completed && Utc::now() >= scheduled_at`
  - `ScheduledTask::mark_completed()` (Line 228-231): Sets completed = true, completed_at = Some(Utc::now())
  - `ScheduledTask::mark_failed()` (Line 233-237): Sets completed = true, error = Some(msg), completed_at = Some(Utc::now())
  - `SchedulerState::due_tasks()` (Line 267-269): Returns `Vec<&ScheduledTask>` filtered by is_due()
  - `SchedulerState::add_task()` (Line 248-253): **Deduplication logic** - removes existing active task for same package_id before adding
  - `SchedulerState::cleanup_old_tasks()` (Line 283-295): Keeps last 50 completed, removes rest

### 2. Configuration & Persistence
- **File:** `src/models/config.rs`
- **Config field:** Line 116 (`pub scheduler: SchedulerState`)
- **Load/Save functions:**
  - `Config::config_path()` (Line 468-470): Returns `~/.config/linget/config.toml`
  - `Config::load()` (Line 472-484): Reads TOML, deserializes, returns default if missing
  - `Config::save()` (Line 486-493): Serializes via `toml::to_string_pretty()`, writes atomically

### 3. Runtime Scheduling Loop
- **File:** `src/ui/relm_app.rs` (5243 lines)
- **App model:** Line 284+ (`pub struct AppModel { ... }`)
- **Startup timer:** Line 1806-1812
  ```rust
  glib::timeout_add_local(Duration::from_secs(60), move || {
      sender_scheduler.input(AppMsg::CheckScheduledTasks);
      glib::ControlFlow::Continue
  })
  ```
  - **Frequency:** Every 60 seconds, continuously repeats
  - **Dependency:** `glib::timeout_add_local()` requires active GTK event loop

### 4. Message-Driven Execution
- **Message enum:** Line 129-280 (`pub enum AppMsg`)
- **Scheduler messages:**
  - Line 259: `ScheduleTask(crate::models::ScheduledTask)` - Create single task
  - Line 260: `ScheduleBulkTasks(Vec<crate::models::ScheduledTask>)` - Create multiple
  - Line 261: `CheckScheduledTasks` - Poll for due tasks (fired by 60s timer)
  - Line 262: `ExecuteScheduledTask(String)` - Run specific task by ID
  - Lines 263-266: `ScheduledTaskCompleted { task_id, package_name }`
  - Lines 267-271: `ScheduledTaskFailed { task_id, package_name, error }`
  - Line 272: `CancelScheduledTask(String)`

### 5. Handlers (in `fn update()` method, Line 1869+)

#### ScheduleTask Handler (Line 3907-3925)
```rust
AppMsg::ScheduleTask(task) => {
    config.scheduler.add_task(task);           // Add to in-memory state
    config.save();                             // LINE 3914 - PERSIST
    sender.input(AppMsg::CheckScheduledTasks); // LINE 3924 - trigger check
}
```

#### ScheduleBulkTasks Handler (Line 3927-3961)
```rust
AppMsg::ScheduleBulkTasks(tasks) => {
    for task in tasks {
        config.scheduler.add_task(task);       // Add each
    }
    config.save();                             // LINE 3935 - PERSIST ALL
    sender.input(AppMsg::CheckScheduledTasks); // LINE 3960 - trigger check
}
```

#### CheckScheduledTasks Handler (Line 3963-3978)
```rust
AppMsg::CheckScheduledTasks => {
    if self.tasks_data.running_task_id.is_some() {
        return;                                // EXCLUSION: skip if busy
    }
    
    let due_tasks = config.scheduler.due_tasks();  // LINE 3970 - Filter
    due_tasks.sort_by_key(|task| task.scheduled_at);
    if let Some(task_id) = due_tasks.first().map(|t| t.id.clone()) {
        sender.input(AppMsg::ExecuteScheduledTask(task_id)); // LINE 3976
    }
}
```

#### ExecuteScheduledTask Handler (Line 3980-4076)
```rust
AppMsg::ExecuteScheduledTask(task_id) => {
    // 1. Fetch task object (Line 3985-3993)
    let task = config.scheduler.tasks.iter().find(|t| t.id == task_id);
    
    // 2. Set exclusive flag (Line 4000)
    self.tasks_data.running_task_id = Some(task_id.clone());
    
    // 3. Spawn async operation (Line 4009)
    glib::spawn_future_local(async move {
        let packages = manager.list_all_installed().await?;
        let pkg = packages.iter().find(|p| p.id() == task.package_id);
        
        // 4. Execute operation (Lines 4018-4029)
        let result = match task.operation {
            ScheduledOperation::Update => manager.update(pkg).await,
            ScheduledOperation::Install => manager.install(pkg).await,
            ScheduledOperation::Remove => manager.remove(pkg).await,
        };
        
        // 5. Mark completion (Lines 4034-4073)
        match result {
            Ok(_) => {
                config.scheduler.tasks.iter_mut()
                    .find(|t| t.id == task_id_clone)
                    .map(|t| t.mark_completed());
                config.scheduler.cleanup_old_tasks(); // LINE 4045
                config.save();                         // LINE 4046
                sender.input(AppMsg::ScheduledTaskCompleted {...});
            }
            Err(e) => {
                config.scheduler.tasks.iter_mut()
                    .find(|t| t.id == task_id_clone)
                    .map(|t| t.mark_failed(e.to_string()));
                config.save();                         // LINE 4064
                sender.input(AppMsg::ScheduledTaskFailed {...});
            }
        }
    });
}
```

#### Completion Handlers
- `ScheduledTaskCompleted` (Line 4078-4098): Clear running_task_id, notify, reload packages, re-check
- `ScheduledTaskFailed` (Line 4100-4121): Clear running_task_id, notify error, re-check
- `CancelScheduledTask` (Line 4123-4135): Remove task, save, re-check

---

## Serialization Format Details

### TOML Format (as written to ~/.config/linget/config.toml)

```toml
[scheduler]
tasks = [
    # Each task is an inline table in the array
    # Fields in order from struct:
    # - id: UUID v4 string
    # - package_id: "source:package_name" (e.g., "apt:vim")
    # - package_name: human readable (e.g., "vim")
    # - source: enum string (Apt, Dnf, Pacman, Flatpak, Snap, Npm, Pip, etc.)
    # - operation: enum string ("Update" | "Install" | "Remove")
    # - scheduled_at: ISO8601 UTC datetime (e.g., "2025-03-08T22:00:00Z")
    # - created_at: ISO8601 UTC datetime
    # - completed: boolean (true | false)
    # - completed_at: ISO8601 UTC or null
    # - error: string or null
]

# Example with one task:
[[scheduler.tasks]]
id = "550e8400-e29b-41d4-a716-446655440000"
package_id = "apt:vim"
package_name = "vim"
source = "Apt"
operation = "Update"
scheduled_at = 2025-03-08T22:00:00Z
created_at = 2025-03-07T14:30:00Z
completed = false
completed_at = null
error = null
```

### DateTime Serialization
- Format: `chrono::DateTime<Utc>` serializes via serde to ISO8601
- Example: `2025-03-08T22:00:00Z` (UTC timezone always 'Z')
- Parsing: systemd uses same ISO8601 format in `.timer` OnCalendar directives

---

## Critical Execution Flow

```
┌─────────────────────────────────────────────────┐
│ APP STARTUP (relm_app.rs:1806)                 │
│ - Register 60-second timeout                   │
│ - Call CheckScheduledTasks immediately         │
└────────────┬────────────────────────────────────┘
             │
             ↓
       ┌──────────────────────────────────────────┐
       │ Every 60 seconds                         │
       │ glib::timeout_add_local callback fires   │
       │ AppMsg::CheckScheduledTasks sent         │
       └────────────┬─────────────────────────────┘
                    │
                    ↓
         ┌─────────────────────────────┐
         │ CheckScheduledTasks (L3963) │
         │ - Filter: due_tasks()       │
         │ - Guard: running_task_id    │
         │ - If any due:               │
         │   Dispatch ExecuteScheduledTask
         └────────────┬────────────────┘
                      │
                      ↓
         ┌─────────────────────────────┐
         │ ExecuteScheduledTask (L3980)│
         │ - Lock: running_task_id     │
         │ - spawn_future_local(async) │
         │ - Execute pkg operation     │
         │ - Mark: completed/failed    │
         │ - Save config to disk       │
         │ - Send callback             │
         └────────────┬────────────────┘
                      │
                      ↓
         ┌─────────────────────────────┐
         │ ScheduledTaskCompleted or   │
         │ ScheduledTaskFailed         │
         │ - Clear running_task_id     │
         │ - Show toast/notification   │
         │ - Re-trigger CheckScheduled │
         └────────────┬────────────────┘
                      │
                      └──→ [Loop back to 60s timer]
```

---

## Where App Must Stay Open

1. **glib::timeout_add_local() callback** (Line 1808)
   - Only fires while GTK event loop is running
   - Stops immediately when app exits
   - No persistence across restarts

2. **glib::spawn_future_local()** (Line 4009)
   - Schedules async work in glib main context
   - Requires active context until await completes
   - Can't work in background daemon

3. **ComponentSender** (implicit)
   - Relm4 message passing uses glib channels
   - Messages only processed while event loop active

---

## Assumptions That Fail

| Scenario | Current Behavior | Systemd Alternative |
|----------|------------------|-------------------|
| App closes at 9:50 PM | Task scheduled for 10 PM never runs | systemd timer wakes system/user session |
| System suspends | No polling while suspended | systemd timer resumes with OnBootSec after wake |
| Package manager needs root | Request goes through sudo (interactive) | Service can run as root or use sudo without tty |
| 60s precision insufficient | Tasks may execute 0-60s late | systemd timer can be 1s precise |
| Multiple tasks queued | Only one executes per timer cycle | systemd could handle multiple via queue depth |

---

## Package Manager Integration

**File:** `src/backend/` (not directly examined in audit)

Current execution calls:
```rust
manager.list_all_installed().await    // Find task's package
manager.update(pkg).await             // Install/update/remove
manager.install(pkg).await
manager.remove(pkg).await
```

**Constraint:** PackageManager may have GTK or relm4 couplings that prevent headless execution.

---

## Key Statistics

| Metric | Value |
|--------|-------|
| Core model lines | 382 (scheduler.rs) |
| Config storage | TOML (human-readable) |
| Poll interval | 60 seconds |
| Max retained completed tasks | 50 |
| Max tasks per package | 1 (active) |
| Task ID generation | UUID v4 |
| Datetime precision | UTC, ISO8601 |
| Persistence | Full config.save() on each change |
| Concurrency | Single-threaded (running_task_id guard) |
| App dependency | GTK event loop (glib::timeout) |

