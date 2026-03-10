# LinGet Tauri DX 9.6: Frontend-Backend Decomposition Analysis

**Date**: March 2025  
**Focus**: Identifying stable IPC boundaries for architectural refactoring  
**Status**: Architecture mapped, decomposition roadmap defined

---

## Executive Summary

The linget-tauri application (React + Tauri) has a **well-defined, compact IPC surface** suitable for decomposition:

- **13 active commands** (frontend → backend)
- **3 active events** (backend → frontend)
- **2 dead commands** (safe to remove)
- **3 dead events** (safe to remove)

All IPC contracts are **explicit and stable** for the duration of refactoring. Backend can be decomposed into `services/` layer without any frontend changes.

---

## Key Findings

### 1. IPC Surface is Minimal & Explicit
- Command naming: Direct one-to-one mapping to domain actions
- Event naming: Clear, centralized emission points
- Type contracts: Well-defined JSON schemas for Package, SourceInfo, SettingsData

### 2. Natural Service Boundaries Exist
Four distinct service domains identified:
1. **PackageService**: Query & mutation of packages (list, search, install, remove, update)
2. **OperationService**: Event emission & operation tracking (progress, logs, batch)
3. **SettingsService**: Persistence & configuration (load, save, apply)
4. **SourceService**: Source enumeration & filtering (availability checks)

Each is **independently testable** and **reusable** without breaking IPC.

### 3. Dead Code is Easily Identifiable
Safe to remove:
- `get_package_info` command (stub, never invoked)
- `get_operation_status` command (never invoked)
- `operation-log` event (listener defined, never emitted)
- `update-progress` event (emitted, never listened)
- `batch-update-started` event (emitted, never listened)

---

## Proposed Architecture (Post-Refactor)

```
src-tauri/src/
├── main.rs                 (50 lines: setup only)
├── state.rs                (70 lines: AppState, AppSettings)
├── handlers/               (200 lines total)
│   ├── package_commands.rs
│   ├── operation_commands.rs
│   └── settings_commands.rs
├── services/               (400 lines total)
│   ├── package_service.rs
│   ├── operation_service.rs
│   ├── settings_service.rs
│   └── source_service.rs
└── utils/                  (100 lines total)
    ├── conversion.rs
    └── error.rs
```

**Same codebase size, better organization** (685 lines unchanged, just reorganized).

---

## Critical Stability Guarantees

### MUST NOT CHANGE during refactoring:
✅ **Command names**: `list_installed_packages`, `install_package`, `search_packages`, etc.  
✅ **Event names**: `operation-progress`, `batch-update-progress`, `batch-update-completed`  
✅ **JSON schemas**: Package fields, SourceInfo fields, SettingsData fields  
✅ **Status/source strings**: `"installed"`, `"APT"`, `"Flatpak"`, etc.  
✅ **Operation ID format**: `"{type}-{name}"` (used in event payloads)  

### CAN CHANGE (internal):
✅ AppState structure  
✅ Handler logic location (move to services)  
✅ Operation state tracking implementation  
✅ Conversion logic organization  

### FRONTEND IMPACT:
**Zero** — No changes required to any frontend code. IPC surface identical before & after.

---

## Implementation Path (4 Phases)

### Phase 1: Extract Utilities
- Move `package_to_json()`, `enabled_sources_from_settings()` to `utils/`
- Add common error types
- **Duration**: 1-2 hours | **Risk**: Minimal

### Phase 2: Extract Services
- Create `services/settings_service.rs`
- Create `services/source_service.rs`
- Create `services/operation_service.rs` (event abstraction)
- Create `services/package_service.rs` (main logic)
- Update handlers to delegate
- **Duration**: 2-3 hours | **Risk**: Low (no IPC changes)

### Phase 3: Organize Handlers
- Create `handlers/` module structure
- Move command definitions to appropriate handler files
- All command signatures remain identical
- **Duration**: 1 hour | **Risk**: None (renaming only)

### Phase 4: Clean Dead Code
- Remove unused commands
- Remove unused events
- Update tests
- **Duration**: 30 mins | **Risk**: None (unused code)

**Total**: ~5 hours, **Zero breaking changes**, **All frontends unaffected**

---

## Testing Strategy

### No Frontend Tests Needed
Frontend tests unchanged — IPC contract is stable.

### Backend Integration Tests (New)
```rust
// Verify command still works after refactoring
#[tokio::test]
async fn test_list_installed_packages_from_service() { ... }

// Verify event emission after moving to service
#[tokio::test]
async fn test_operation_progress_emits_correct_payload() { ... }

// Verify settings contract after persistence move
#[tokio::test]
async fn test_settings_save_load_roundtrip() { ... }
```

### Regression Test
- Run frontend against refactored backend
- All commands/events should work identically

---

## Key Insights for DX 9.6

1. **Decomposition = Localization, Not Redesign**
   - The monolithic `main.rs` can be decomposed without changing the public API
   - Services are internal implementation details
   - Frontend remains blissfully unaware

2. **Service Layer is the Abstraction**
   - `OperationService` centralizes all event emission (easy to swap mechanisms)
   - `PackageService` encapsulates manager interactions (easy to mock, test)
   - `SettingsService` isolates persistence (easy to change storage later)

3. **Operation Tracking is the Most Complex Part**
   - HashMap-based state management with Arc<AtomicBool> + AtomicU64
   - Could be refactored to use `async-channel` or tokio::mpsc
   - But IPC events must remain identical

4. **Type Contracts are the Anchor**
   - serde_json::Value serialization is the IPC wire format
   - As long as JSON keys/types match, internal representation doesn't matter
   - Frontend expects specific field names in specific event payloads

---

## Documentation Files Included

1. **IPC_SURFACE_ANALYSIS.md** (15 KB)
   - Complete command/event listing
   - Type schemas
   - State management patterns
   - Integration testing notes

2. **IPC_MAPPING_VISUAL.txt** (14 KB)
   - ASCII diagrams of data flow
   - Command channel visualization
   - Event channel visualization
   - Operation state tracking diagram

3. **DECOMPOSITION_ROADMAP.md** (15 KB)
   - Detailed service layer code (examples)
   - File structure after refactoring
   - Migration steps
   - Stability checklist for reviewers

4. **IPC_QUICK_REF.txt** (11 KB)
   - One-page reference
   - Active vs dead code
   - Stability rules
   - Quick lookup tables

---

## Next Steps

1. **Code Review**: Validate IPC surface analysis against actual code
2. **Design Review**: Approve proposed service layer structure
3. **Implementation**: Execute 4-phase migration plan
4. **Testing**: Run regression tests with frontend
5. **Merge**: Zero breaking changes, all systems green

---

## Contact & Questions

For questions about:
- **IPC Contract Details**: See IPC_SURFACE_ANALYSIS.md
- **Visual Data Flow**: See IPC_MAPPING_VISUAL.txt
- **Code Implementation**: See DECOMPOSITION_ROADMAP.md
- **Quick Lookup**: See IPC_QUICK_REF.txt

---

**Conclusion**: The linget-tauri codebase is well-structured for decomposition. The IPC surface is explicit, minimal, and stable. Backend refactoring can proceed with zero frontend impact.
