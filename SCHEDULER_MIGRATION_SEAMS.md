# LinGet Scheduler → Systemd Migration Seams

## Architecture Mismatch Summary

```
CURRENT (UI-Bound)           SYSTEMD (Daemon-Based)
═══════════════════          ════════════════════

Trigger:                     Trigger:
  glib::timeout (60s)          systemd.timer (OnBootSec + OnUnitActiveSec)
        ↓                              ↓
  CheckScheduledTasks        systemd service ExecStart=/usr/bin/linget schedule run
        ↓                              ↓
  Load config in-memory      Load config from disk
  Keep running_task_id       No state needed in service
        ↓                              ↓
  spawn_future_local()       Direct tokio::main() or async block
  (glib event loop)          (independent process)
        ↓                              ↓
  mark_completed()           mark_completed()
  config.save()              config.save() [WITH LOCK]
        ↓                              ↓
  Toast + notification       D-Bus notification (headless-safe)
        ↓                              ↓
  UI updates                 Journal log + exit(0)
```

---

## Code Seams: What Must Change

### SEAM 1: Entry Point
**Current:** App is entry point
```rust
// src/main.rs:1
fn main() {
    detect_run_mode()  // GUI vs TUI vs CLI
    match RunMode::Gui => gui_main()
}
```

**Change needed:**
```rust
// Add new CLI command
#[derive(Subcommand)]
pub enum Commands {
    Schedule {
        #[command(subcommand)]
        cmd: ScheduleCommand,
    },
    // ...existing...
}

#[derive(Subcommand)]
pub enum ScheduleCommand {
    Run,           // Execute all due tasks
    List,          // List pending/completed
    Cancel { id: String },
    // ...
}

// src/cli/commands/schedule.rs (NEW)
pub async fn handle_schedule_run() -> anyhow::Result<()> {
    // Load config WITHOUT GTK
    let mut config = Config::load();
    let pm = PackageManager::new().await?;  // Must not require GTK
    
    let due = config.scheduler.due_tasks();
    for task in due {
        execute_task(&pm, &task).await?;
        task.mark_completed();
        config.save()?;  // Save after each (or batch?)
    }
    Ok(())
}
```

**File locations to modify:**
- `src/cli/mod.rs` - Add ScheduleCommand variant (line 10+)
- `src/cli/commands/mod.rs` - Add schedule.rs module
- `src/cli/commands/schedule.rs` - NEW: Implementation
- `src/main.rs` - Route to schedule handler

---

### SEAM 2: Synchronization Lock
**Current:** None (single user, GUI only)
```rust
// src/ui/relm_app.rs:3913 - direct access
config.scheduler.add_task(task);
config.save();  // No protection
```

**Change needed:**
```rust
// src/models/config.rs (NEW)
use std::fs::File;
use std::path::Path;
use fs2::FileExt;  // Add to Cargo.toml

pub struct ConfigLock {
    _lock: File,
}

impl ConfigLock {
    pub fn acquire() -> anyhow::Result<Self> {
        let lock_path = Self::lock_path();
        let lock = File::create(&lock_path)?;
        lock.try_lock_exclusive()
            .context("Failed to acquire config lock")?;
        Ok(ConfigLock { _lock: lock })
    }
    
    fn lock_path() -> PathBuf {
        Config::config_dir().join("config.lock")
    }
}

// Usage in both GUI and CLI:
// src/ui/relm_app.rs:3912
let _lock = ConfigLock::acquire()?;
let mut config = self.config.borrow_mut();
config.scheduler.add_task(task);
config.save()?;
drop(_lock);  // Release on scope exit

// src/cli/commands/schedule.rs
let _lock = ConfigLock::acquire()?;
let mut config = Config::load();
// ... modify ...
config.save()?;
drop(_lock);
```

**File locations:**
- `src/models/config.rs` - Add ConfigLock struct + methods
- `Cargo.toml` - Add `fs2` dependency
- `src/ui/relm_app.rs:3907` - Wrap ScheduleTask handler
- `src/ui/relm_app.rs:3927` - Wrap ScheduleBulkTasks handler
- `src/ui/relm_app.rs:3980` - Wrap ExecuteScheduledTask handler
- `src/cli/commands/schedule.rs` - Use lock in run()

---

### SEAM 3: PackageManager Independence
**Current:** Coupled to Arc<Mutex<PackageManager>>
```rust
// src/ui/relm_app.rs:400+
struct AppModel {
    package_manager: Arc<tokio::sync::Mutex<PackageManager>>,
    // ...
}

// Line 4018-4028: Async closure captures pm
let pm = self.package_manager.clone();
glib::spawn_future_local(async move {
    let manager = pm.lock().await;
    manager.update(pkg).await
});
```

**Check:** Does PackageManager have GTK imports?
```bash
grep -r "use gtk\|use relm\|use glib\|use libadwaita" src/backend/
# If yes, must refactor to pure backend module
```

**If coupled to UI:** Extract to headless module
```rust
// src/backend/mod.rs
pub use package_manager::PackageManager;  // Re-export

// CLI can now do:
// src/cli/commands/schedule.rs
use crate::backend::PackageManager;
let pm = PackageManager::new().await?;
// No GTK dependency required
```

**Files to check/modify:**
- `src/backend/mod.rs` - Verify no UI imports
- `src/backend/package_manager.rs` - If exists, check for GTK
- `Cargo.toml` - Ensure backend doesn't depend on gtk4/libadwaita

---

### SEAM 4: Notification System
**Current:** GTK-based (libadwaita)
```rust
// src/ui/relm_app.rs:4088-4090
if self.config.borrow().show_notifications {
    notifications::send_task_completed_notification(&package_name);
}

// src/ui/notifications.rs (assumed)
// Uses GTK desktop notification API
```

**Problem:** Doesn't work in headless systemd service

**Solution:** Use zbus (D-Bus) directly
```rust
// src/backend/notifications.rs (NEW - pure module, no UI)
use zbus::dbus_proxy;

#[dbus_proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: &std::collections::HashMap<&str, zbus::zvariant::Value>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
}

pub async fn notify_task_completed(pkg: &str) -> anyhow::Result<()> {
    let connection = zbus::Connection::session().await?;
    let proxy = NotificationsProxy::new(&connection).await?;
    
    proxy.notify(
        "LinGet",
        0,
        "software-update-available-symbolic",
        &format!("Update completed: {}", pkg),
        "Task completed successfully",
        &[],
        &Default::default(),
        5000,  // 5s timeout
    ).await?;
    Ok(())
}

// Usage in both GUI and CLI:
// src/ui/relm_app.rs:4089
notifications::notify_task_completed(&package_name).await.ok();

// src/cli/commands/schedule.rs
notifications::notify_task_completed(&task.package_name).await.ok();
```

**Files:**
- `src/ui/notifications.rs` - Refactor to wrap headless impl
- `src/backend/notifications.rs` - NEW pure module
- `Cargo.toml` - Add `zbus` dependency
- Update imports in relm_app.rs and schedule.rs CLI command

---

### SEAM 5: glib::spawn_future_local Replacement
**Current:** Uses glib event loop context
```rust
// src/ui/relm_app.rs:4009-4074
glib::spawn_future_local(async move {
    // Runs in glib main context
});
```

**In CLI:** Must use tokio runtime directly
```rust
// src/cli/commands/schedule.rs
pub async fn handle_schedule_run() -> anyhow::Result<()> {
    // Already in async context from main()
    // No glib::spawn_future_local needed
    
    let mut config = Config::load();
    let pm = PackageManager::new().await?;
    
    for task in config.scheduler.due_tasks() {
        // Directly await, no spawn needed
        execute_task_async(&pm, &task).await?;
        task.mark_completed();
        config.save()?;
    }
    
    Ok(())
}

async fn execute_task_async(pm: &PackageManager, task: &ScheduledTask) 
    -> anyhow::Result<()> 
{
    // Extracted from relm_app.rs:4017-4032
    let packages = pm.list_all_installed().await?;
    let pkg = packages.iter().find(|p| p.id() == task.package_id)
        .ok_or_else(|| anyhow::anyhow!("Package not found"))?;
    
    match task.operation {
        ScheduledOperation::Update => pm.update(pkg).await,
        ScheduledOperation::Install => pm.install(pkg).await,
        ScheduledOperation::Remove => pm.remove(pkg).await,
    }?;
    
    Ok(())
}
```

**Files:**
- `src/cli/commands/schedule.rs` - Implement async without glib
- `src/main.rs` - Ensure tokio runtime is set up for CLI

---

### SEAM 6: Systemd Service Files
**Current:** None

**Create new files:**

`~/.config/systemd/user/linget-schedule.service`
```ini
[Unit]
Description=LinGet Scheduled Task Executor
After=network.target
Documentation=man:linget(1)

[Service]
Type=oneshot
ExecStart=/usr/bin/linget schedule run
StandardOutput=journal
StandardError=journal
# No user= needed for user service

[Install]
WantedBy=default.target
```

`~/.config/systemd/user/linget-schedule.timer`
```ini
[Unit]
Description=LinGet Scheduled Task Timer
Requires=linget-schedule.service
Documentation=man:linget(1)

[Timer]
# Check every minute (vs current 60s from glib timeout)
OnBootSec=1min
OnUnitActiveSec=1min
# Optional: use specific times for predictability
# OnCalendar=*-*-* *:*:00   # Every minute at :00 seconds

[Install]
WantedBy=timers.target
```

**User must enable:**
```bash
systemctl --user daemon-reload
systemctl --user enable linget-schedule.timer
systemctl --user start linget-schedule.timer
```

---

## Integration Matrix

### GUI (Current, Unchanged)
```
relm_app.rs:1806      ← glib::timeout_add_local (60s)
relm_app.rs:3907      ← AppMsg::ScheduleTask (+ ConfigLock)
relm_app.rs:3980      ← AppMsg::ExecuteScheduledTask (+ ConfigLock)
notifications.rs      ← zbus-backed notify_task_completed()
```

### CLI Schedule Run (New)
```
main.rs               ← detect CLI schedule run command
cli/mod.rs            ← ScheduleCommand enum
cli/commands/schedule.rs ← async handle_schedule_run() function
  ├─ ConfigLock::acquire()
  ├─ Config::load()
  ├─ scheduler.due_tasks()
  ├─ execute_task_async() [extracted from relm_app]
  ├─ task.mark_completed() / mark_failed()
  ├─ config.save()
  ├─ notifications::notify_*()  [zbus-backed]
  └─ exit(0)
```

### Systemd Timer (New)
```
linget-schedule.timer  ← OnBootSec=1min + OnUnitActiveSec=1min
  └─ linget-schedule.service
      └─ ExecStart=/usr/bin/linget schedule run
          └─ (spawns CLI process above)
```

---

## Lock Acquisition Timing

### Scenario: GUI + systemd both active

```
Timeline:
T=0ms   systemd timer fires → systemd service starts
        ExecStart: /usr/bin/linget schedule run
        
T=5ms   CLI process: ConfigLock::acquire() → WAIT (blocks)
        GUI process has lock? Check...
        
T=10ms  GUI on timer check, calls AppMsg::ScheduleTask
        GUI grabs ConfigLock → ACQUIRED (CLI blocked)
        GUI modifies config in-memory
        
T=20ms  GUI config.save() → File updated on disk
        GUI ConfigLock released
        
T=21ms  CLI wakes up, ConfigLock::acquire() → ACQUIRED
        CLI Config::load() reads disk (sees GUI's updates)
        CLI checks due_tasks() (only includes due ones)
        CLI executes (no conflict, task already in GUI's state)
        
T=500ms CLI config.save() → File updated
        CLI exit(0)
```

**Result:** No data corruption, both respect config file, minimal lock contention.

---

## Rollback Plan

If systemd migration fails:
1. Keep glib::timeout_add_local() running in GUI (always works)
2. Add CLI command that CLI can run manually: `linget schedule run`
3. Don't require systemd; document as optional for headless users
4. GUI remains single source of truth for scheduling

---

## Testing Checklist

- [ ] `linget schedule list` shows pending + completed tasks
- [ ] `linget schedule run` executes all due tasks, exits cleanly
- [ ] GUI + CLI don't corrupt config when both modify simultaneously
- [ ] Systemd timer wakes up and executes service
- [ ] Completed tasks persist in config after systemd service runs
- [ ] Notifications work both in GUI and headless systemd
- [ ] Failed tasks recorded with error message
- [ ] `systemctl --user status linget-schedule.timer` shows healthy state
- [ ] App restart doesn't lose pending tasks (reload from config)
- [ ] Lock timeout prevents deadlock (if GUI crashes with lock held)

