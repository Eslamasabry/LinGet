# LinGet Tauri Architecture Documentation - Decomposition Analysis

This directory contains comprehensive analysis of the linget-tauri frontend-backend IPC surface for DX 9.6 refactoring work.

## 📋 Documentation Index

### 1. **DX9.6_DECOMPOSITION_SUMMARY.md** ⭐ START HERE
**2-page executive summary** of the entire analysis.

Key takeaways:
- 13 active commands + 3 active events = stable IPC surface
- 4 natural service boundaries identified
- Zero frontend impact for backend refactoring
- 4-phase migration plan with ~5 hours total effort

**Read this first** to understand the big picture.

---

### 2. **IPC_QUICK_REF.txt** 📌 QUICK LOOKUP
**1-page quick reference card** for developers.

Contains:
- Table of all active commands (13)
- Table of all active events (3)
- Dead code identification (safe to remove)
- Stability rules (MUST preserve vs CAN change)
- Proposed service structure
- Operation ID naming convention

**Use this** when you need a quick fact check during implementation.

---

### 3. **IPC_SURFACE_ANALYSIS.md** 🔍 DETAILED REFERENCE
**Comprehensive technical reference** (15 KB).

Contains:
- Complete command handler listing with signatures
- Event payload definitions
- Shared type definitions (TypeScript + Rust)
- State management patterns
- Service decomposition opportunities
- Integration testing implications
- Decomposition boundaries & hooks/services structure

**Reference this** when implementing specific services or understanding patterns.

---

### 4. **IPC_MAPPING_VISUAL.txt** 📊 DATA FLOW DIAGRAMS
**ASCII diagrams and visual mappings** (14 KB).

Contains:
- State management data flow (Frontend ↔ Backend)
- Command channel visualization (13 commands)
- Event channel visualization (6 events)
- Operation state tracking diagram
- Shared type contracts (JSON schemas)
- Stability requirements summary

**Use these** to understand how data flows through the system.

---

### 5. **DECOMPOSITION_ROADMAP.md** 🛣️ IMPLEMENTATION GUIDE
**Detailed migration roadmap** with code examples (15 KB).

Contains:
- Proposed service layer code (PackageService, OperationService, etc.)
- File structure after refactoring
- 4 migration phases with effort estimates
- Stability checklist for reviewers
- Frontend impact analysis
- Migration steps (step-by-step)

**Follow this** when actually implementing the refactoring.

---

## 🎯 How to Use This Documentation

### For Architects / Team Leads:
1. Read **DX9.6_DECOMPOSITION_SUMMARY.md** (2 min)
2. Review **IPC_MAPPING_VISUAL.txt** for data flow understanding (5 min)
3. Approve the 4-phase migration plan from **DECOMPOSITION_ROADMAP.md**

### For Developers Implementing Phase 1 (Utilities):
1. Check **IPC_QUICK_REF.txt** for stability rules
2. Reference **IPC_SURFACE_ANALYSIS.md** § "Decomposition Boundaries"
3. Follow examples in **DECOMPOSITION_ROADMAP.md** § "Proposed Services Architecture"
4. Use **IPC_MAPPING_VISUAL.txt** for data flow confirmation

### For Developers Implementing Phase 2 (Services):
1. Start with **DECOMPOSITION_ROADMAP.md** § "Proposed Services Architecture"
2. Use **IPC_SURFACE_ANALYSIS.md** for complete handler signatures
3. Check **IPC_QUICK_REF.txt** for stability guarantees
4. Use **IPC_MAPPING_VISUAL.txt** to verify event emission patterns

### For Code Reviewers:
1. Use **IPC_QUICK_REF.txt** § "CRITICAL STABILITY RULES"
2. Check **DECOMPOSITION_ROADMAP.md** § "Stability Checklist for Reviewers"
3. Verify no changes to items in **IPC_SURFACE_ANALYSIS.md** § "STABLE IPC SURFACE"

### For QA / Testing:
1. Read **DX9.6_DECOMPOSITION_SUMMARY.md** § "Testing Strategy"
2. Reference **IPC_SURFACE_ANALYSIS.md** § "Integration Testing Implications"
3. Use commands/events from **IPC_MAPPING_VISUAL.txt** for regression testing

---

## 📊 Quick Stats

| Metric | Value |
|--------|-------|
| Frontend-to-Backend Commands | 13 active, 2 dead |
| Backend-to-Frontend Events | 3 active, 3 dead |
| Identified Service Domains | 4 (Package, Operation, Settings, Source) |
| IPC Contract Stability | 100% (no breaking changes required) |
| Frontend Impact | Zero (all refactoring is internal) |
| Estimated Effort | ~5 hours total |
| Code Reorganization Risk | Low (no logic changes, just structure) |

---

## 🔐 Stability Guarantees

### MUST NOT CHANGE (IPC Public API)
- Command names: `list_installed_packages`, `install_package`, etc.
- Event names: `operation-progress`, `batch-update-progress`, etc.
- JSON field names in type schemas
- Status/source enum string representations
- Operation ID format: `{type}-{name}`

### CAN CHANGE (Internal Implementation)
- AppState structure
- Handler function organization
- Operation state tracking implementation
- Conversion logic location
- Manager initialization patterns

### FRONTEND IMPACT
**Zero** — No changes required to React code. IPC surface is identical before and after refactoring.

---

## 🗺️ IPC Surface at a Glance

### Commands (Frontend → Backend)
```
Query:   list_installed_packages, list_available_packages, check_updates, search_packages
Mutate:  install_package, remove_package, update_package, update_all_packages
Control: cancel_operation
Config:  load_settings, save_settings, list_sources, get_backend_sources
```

### Events (Backend → Frontend)
```
operation-progress          (per install/remove/update)
batch-update-progress       (per update_all_packages iteration)
batch-update-completed      (end of update_all_packages)
```

### Shared Types
```
Package              (package info with status)
SourceInfo           (package source with availability)
SettingsData         (user preferences)
OperationProgressPayload (operation tracking)
```

---

## 🚀 Implementation Phases

### Phase 1: Extract Utilities (1-2 hours)
- Move conversion functions (`package_to_json`, etc.)
- Add common error types
- **Risk**: Minimal

### Phase 2: Extract Services (2-3 hours)
- Create `SettingsService`, `SourceService`, `OperationService`, `PackageService`
- Update handlers to delegate
- **Risk**: Low (no IPC changes)

### Phase 3: Organize Handlers (1 hour)
- Create `handlers/` module structure
- Move command definitions
- All signatures remain identical
- **Risk**: None (structure only)

### Phase 4: Clean Dead Code (30 mins)
- Remove unused commands/events
- **Risk**: None (removing unused code)

**Total**: ~5 hours, **Zero breaking changes**

---

## 📞 Troubleshooting

**Q: Will this break the frontend?**  
A: No. The IPC surface (command names, event names, payload schemas) remains identical.

**Q: Which service should I add new functionality to?**  
A: See **IPC_SURFACE_ANALYSIS.md** § "Decomposition Boundaries" for guidelines.

**Q: Can I change the event payload structure?**  
A: No. All event payloads are defined in **IPC_QUICK_REF.txt** and must remain stable.

**Q: What if I need to emit a new event?**  
A: Add it to `OperationService` (see **DECOMPOSITION_ROADMAP.md** § "Operation Service").

**Q: How do I test my refactoring?**  
A: Run the frontend against your refactored backend. All commands/events should work identically.

---

## 📚 Related Files in the Codebase

- **Frontend**: `/ui/src/App.tsx` (1830 lines)
  - Contains all `invoke()` calls and `listen()` setup
  - Type definitions for IPC payloads
  
- **Backend**: `/src-tauri/src/main.rs` (685 lines)
  - Contains all `#[tauri::command]` handlers
  - Event emission logic
  
- **Shared Core**: `../../packages/backend-core` and `../../packages/backends`
  - External crates with Package, PackageSource, PackageManager types

---

## ✅ Validation Checklist

Before merging any refactoring PR:

- [ ] All 13 active commands still callable with same names
- [ ] All 3 active events still emitted with same names
- [ ] Package JSON schema unchanged (all fields present)
- [ ] SourceInfo JSON schema unchanged
- [ ] SettingsData JSON schema unchanged
- [ ] OperationProgressPayload structure preserved
- [ ] Frontend tests pass (run existing test suite)
- [ ] Integration tests added for services (3+ tests minimum)
- [ ] No changes to serde_json serialization logic
- [ ] Dead code marked for removal (separate PR)

---

## 📝 Version History

- **v1.0** (March 2025): Initial analysis, 4-phase decomposition roadmap

---

**Last Updated**: March 7, 2025  
**Analyzed By**: DX 9.6 Architecture Review  
**Status**: Ready for Implementation
