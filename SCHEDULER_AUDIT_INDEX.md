# LinGet Scheduler DX9.5 Audit - Document Index

## Overview
This directory contains a comprehensive audit of LinGet's persistent scheduler implementation and roadmap for systemd user timer/service integration.

**Audit Date:** March 2025  
**Scope:** DX9.5 scheduler feature  
**Auditor Focus:** Code paths, persistence, runtime assumptions, migration seams  

---

## Documents

### 1. `SCHEDULER_AUDIT.md` (Main Report - 483 lines)
**Purpose:** Complete architectural analysis with context

**Sections:**
- Executive Summary (what it is + what it's missing)
- Current Scheduler Architecture (data model, persistence, runtime loop)
- Task Lifecycle with diagram
- UI Integration Points
- Systemd Migration Seams (5 major code changes needed)
- Risk Assessment matrix
- Conclusion with effort estimate

**Best for:** Understanding the full picture, stakeholders, architects

---

### 2. `SCHEDULER_QUICK_REFERENCE.md` (Technical Map - 456 lines)
**Purpose:** Exact file paths, function names, line numbers for developers

**Sections:**
- File Locations & Function Map (with line #s)
  - Data Model (scheduler.rs)
  - Configuration & Persistence (config.rs)
  - Runtime Scheduling Loop (relm_app.rs)
  - Message-Driven Execution
  - Handler Implementations (with pseudo-code)
  - Completion Handlers
- Serialization Format Details (TOML structure)
- Critical Execution Flow (timing diagram)
- Package Manager Integration
- Key Statistics table

**Best for:** Implementation, debugging, code review, IDE jump-to-definition

---

### 3. `SCHEDULER_MIGRATION_SEAMS.md` (Implementation Guide - 371 lines)
**Purpose:** Step-by-step code seams for systemd migration

**Sections:**
- Architecture Mismatch Summary (visual comparison)
- Code Seams 1-6 (what must change + exact how)
  1. Entry Point (new CLI command)
  2. Synchronization Lock (fs2 file locking)
  3. PackageManager Independence (decouple from GTK)
  4. Notification System (migrate to zbus D-Bus)
  5. glib::spawn_future_local Replacement (tokio direct)
  6. Systemd Service Files (unit file templates)
- Integration Matrix (GUI vs CLI vs systemd)
- Lock Acquisition Timing (concurrency safety)
- Rollback Plan
- Testing Checklist (14 items)

**Best for:** Implementation planning, engineers writing the feature, CI/CD setup

---

## Quick Facts

| Aspect | Details |
|--------|---------|
| **Current Storage** | `~/.config/linget/config.toml` (human-readable) |
| **Polling Interval** | 60 seconds (via glib::timeout_add_local) |
| **Task ID** | UUID v4 (generated on create) |
| **Datetime Format** | ISO8601 UTC (e.g., `2025-03-08T22:00:00Z`) |
| **Max Retained Completed** | 50 tasks (oldest purged) |
| **Max Active per Package** | 1 (new replaces old) |
| **Persistence** | Atomic TOML save after each change |
| **Concurrency Model** | Single-threaded (running_task_id guard) |
| **Must-Stay-Open** | YES - glib timeout requires event loop |
| **Systemd Ready** | Serialization format compatible, but requires CLI entrypoint |

---

## Core File Map

```
linget/
├── src/
│   ├── models/
│   │   ├── scheduler.rs ..................... Data model (ScheduledTask, SchedulerState)
│   │   │   ├── ScheduledTask struct (L127)
│   │   │   ├── SchedulerState container (L241)
│   │   │   ├── is_due(), mark_completed(), due_tasks() methods
│   │   │   └── [READY FOR SYSTEMD] ✓ No GTK deps
│   │   │
│   │   └── config.rs ........................ Persistence (load/save)
│   │       ├── Config struct with scheduler field (L116)
│   │       ├── config_path() (L468)
│   │       ├── load() deserializes TOML (L472)
│   │       ├── save() serializes TOML (L486)
│   │       └── [READY FOR SYSTEMD] ✓ Only uses toml crate
│   │
│   ├── ui/
│   │   ├── relm_app.rs ..................... Runtime scheduling (5243 lines)
│   │   │   ├── Startup timer (L1806): glib::timeout_add_local(60s)
│   │   │   ├── AppMsg enum with scheduler variants (L259-272)
│   │   │   ├── ScheduleTask handler (L3907) → add + save
│   │   │   ├── CheckScheduledTasks handler (L3963) → poll due
│   │   │   ├── ExecuteScheduledTask handler (L3980) → run async
│   │   │   ├── ScheduledTaskCompleted handler (L4078)
│   │   │   └── [REQUIRES CHANGES] ✗ GTK-bound execution
│   │   │
│   │   ├── task_queue_view.rs ............ UI display
│   │   │   ├── TaskQueueViewData struct (L12)
│   │   │   ├── retry_failed_task_ids() (L62)
│   │   │   └── build_pending_task_row() rendering (L349)
│   │   │
│   │   └── widgets/schedule_popover.rs .. Schedule UI
│   │       ├── build_schedule_popover() (L13)
│   │       └── ScheduledTask::new() on preset selection (L104)
│   │
│   ├── cli/ ................................ [NEW FILES NEEDED]
│   │   ├── mod.rs (modify) ................ Add ScheduleCommand variant
│   │   ├── commands/mod.rs (modify) ...... Add schedule.rs module
│   │   └── commands/schedule.rs (NEW) .... Execute scheduled tasks headless
│   │
│   └── main.rs (modify) ................... Route CLI schedule command
│
├── Cargo.toml (modify) ..................... Add fs2, zbus dependencies
├── ~/.config/linget/config.toml .......... Persistent storage
│
└── ~/.config/systemd/user/ (NEW files)
    ├── linget-schedule.service
    └── linget-schedule.timer
```

---

## Critical Code Paths

### Data Creation (User → Disk)
```
UI schedule_popover.rs:104
  ↓ ScheduleTask message
relm_app.rs:3907 AppMsg::ScheduleTask
  ├─ add_task() [scheduler.rs:248]
  ├─ save() [config.rs:486] ← PERSISTS
  └─ CheckScheduledTasks → poll loop
```

### Execution (Disk → Package Manager)
```
relm_app.rs:1808 glib::timeout (60s) ← ⚠️ REQUIRES APP RUNNING
  ↓
relm_app.rs:3963 CheckScheduledTasks
  ├─ due_tasks() filter [scheduler.rs:267]
  └─ ExecuteScheduledTask dispatch
    ↓
relm_app.rs:3980 ExecuteScheduledTask
  ├─ spawn_future_local (glib async) ← ⚠️ REQUIRES EVENT LOOP
  └─ manager.{update|install|remove}()
    ↓
relm_app.rs:4043/4062 mark_completed/mark_failed
  ├─ save() [config.rs:486] ← PERSISTS
  └─ ScheduledTaskCompleted message → UI update
```

### Systemd Path (CLI → Package Manager) [PROPOSED]
```
systemd.timer fires (OnBootSec=1min, OnUnitActiveSec=1min)
  ↓
linget-schedule.service ExecStart
  ↓
cli/commands/schedule.rs handle_schedule_run()
  ├─ ConfigLock::acquire() (NEW)
  ├─ Config::load() [config.rs:472]
  ├─ due_tasks() filter [scheduler.rs:267]
  └─ execute_task_async() [NEW]
    ├─ manager.{update|install|remove}()
    ├─ mark_completed/mark_failed()
    ├─ save() [config.rs:486] ← PERSISTS
    └─ notify_*() [NEW zbus-backed]
  ↓
exit(0)
```

---

## Key Assumptions That Fail

| Requirement | Current | Fails When | Systemd Fix |
|-----------|---------|-----------|------------|
| Periodic polling | glib timeout 60s | App closes | systemd timer with accurate timestamp |
| Task execution | glib spawn_future_local | Event loop inactive | tokio runtime in service |
| Notifications | GTK API | Running headless | zbus D-Bus (works everywhere) |
| Concurrent access | None (single GUI) | Multiple processes | ConfigLock file locking |
| Resume after suspend | None | System wakes | systemd timer supports RTC wake |

---

## Next Steps by Role

### Product Manager
- Read: SCHEDULER_AUDIT.md (Executive Summary)
- Understand: Why app must stay open, what systemd unlocks
- Decision: Is headless scheduling a priority?

### Architect
- Read: SCHEDULER_MIGRATION_SEAMS.md (full document)
- Review: Integration Matrix and Lock Acquisition Timing
- Assess: Impact on UI, CLI, and systemd integration
- Approve: 6 major seams before engineering starts

### Backend Engineer
- Reference: SCHEDULER_QUICK_REFERENCE.md for all line numbers
- Follow: SCHEDULER_MIGRATION_SEAMS.md seams 1-6 in order
- Use: Testing Checklist for validation
- Integrate: With CI/CD for systemd unit installation

### QA Engineer
- Run: Testing Checklist (14 scenarios)
- Test: GUI still works unchanged
- Test: CLI can run standalone
- Test: GUI + systemd running simultaneously (lock contention)
- Test: Systemd timer reliability over 7 days

---

## Document Stats

| Document | Lines | Focus | Audience |
|----------|-------|-------|----------|
| SCHEDULER_AUDIT.md | 483 | What + Why | Everyone |
| SCHEDULER_QUICK_REFERENCE.md | 456 | Where + Exact | Engineers |
| SCHEDULER_MIGRATION_SEAMS.md | 371 | How + Plan | Architects + Engineers |
| SCHEDULER_AUDIT_INDEX.md (this) | 304 | Navigation | Everyone |

**Total:** 1,614 lines of documentation  
**Code locations:** 27 exact file:line references  
**Diagrams:** 5 ASCII visualizations  

---

## Validation Checklist

- [x] Audit covers all scheduler files mentioned
- [x] Data model serialization format documented with examples
- [x] Runtime polling mechanism identified (glib 60s timeout)
- [x] All message handlers traced with line numbers
- [x] Persistence verified (TOML, atomic save)
- [x] Systemd migration seams enumerated (6 major changes)
- [x] Lock strategy designed (fs2 file locking)
- [x] Notification refactoring planned (zbus D-Bus)
- [x] CLI entrypoint specified
- [x] Unit file templates provided
- [x] Risk matrix completed
- [x] Testing plan included (14 scenarios)

---

## Related Docs in Repo

- `README.md` - Project overview
- `PLAN.md` - Feature roadmap
- `CONTRIBUTING.md` - Development setup
- `Cargo.toml` - Dependencies (will need fs2, zbus added)

---

Generated: March 2025  
Status: Complete Audit Ready for Implementation  
