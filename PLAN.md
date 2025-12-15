# LinGet UI/UX Roadmap

This document tracks planned UI/UX improvements and the order to implement them.

## Goals
- Feel fast, calm, and “premium” (minimal friction, clear hierarchy, consistent spacing).
- Make power actions obvious without clutter (updates, installs, filters).
- Keep the UI responsive; avoid work on the GTK main thread.

## Workstreams

### 1) Sources & Filtering
- [x] Redesign sidebar into a **Providers** enable/disable list (availability + enabled state).
- [x] Sort providers by **available → enabled → name**; dim + disable unavailable providers.
- [x] Move "filter by source" into the **top toolbar popover** (Source: All / Source: X).
- [x] Add provider install hints (e.g., "Install `pipx`").
- [ ] Make selection states consistent across views (All/Updates/Discover).
- [x] Persist filter state (optional) per session.

### 2) Package List Rows (Polish)
- [x] Refine row layout (icon frame, spacing, chips for version/source).
- [x] Hover affordances (show actions on hover, reduce persistent chevrons).
- [x] Clamp long version chips (ellipsis + tooltip).
- [x] Align suffix actions (consistent icon sizing).
- [x] Inline per-row operation progress for the active package.
- [x] Live row state refresh (Update → Updated, Install → Installed).

### 3) Discover Experience
- [ ] Source tabs (All / System / Flatpak / Snap / Dev tools).
- [ ] Inline install from results (no modal required).
- [ ] Result caching + debounced search per provider.
- [ ] Better result badges (source/type/status).

### 4) Updates Experience
- [ ] “Summary bar” with Installed / Updates / Providers and quick jump actions.
- [ ] Unified update workflow (Update All, Update Selected, retry failures).
- [ ] Better error presentation (group failures; actionable next steps).

### 5) Notifications & Background Work
- [x] Background refresh scheduling (already present; refine UX).
- [x] Optional desktop notifications (respect config and distro capabilities).
- [x] System tray icon with update count badge and quick actions.
- [ ] Avoid noisy toasts; show a single consolidated "activity" surface.

### 6) Command Center & Error UX
- [x] Replace many toasts with an expandable **Command Center** panel.
- [x] “Copy command” actions for privilege-required failures.
- [x] Add timestamps + retry actions for recent operations.
- [x] Show active operations section.
- [x] Structured diagnostics export (already present; expand content).

### 7) Preferences & Onboarding
- [ ] Group providers by category (System / Sandboxed / Dev tools).
- [ ] Show “Detected / Not detected” with install hints.
- [ ] First-run onboarding if few providers are available/enabled.

### 8) Keyboard & Accessibility
- [x] `/` focuses search, `Esc` exits selection mode/clears search.
- [x] `Enter` opens details, `U` updates selected, `Delete` removes selected (in selection mode).
- [ ] Ensure focus rings + accessible names for controls.

### 9) Downgrade / Revert
- [x] Add downgrade API (backend + UI).
- [x] Snap: **Revert** to previous revision (`snap revert`).
- [x] DNF: `dnf downgrade` (with version selection later).
- [x] APT: install specific older version (`apt install pkg=version`) with version picker.
- [ ] Flatpak: commit pinning / branch selection (advanced).
- [x] Npm/pip/pipx/cargo/dart: install specific version (acts as downgrade).
- [x] Conda/mamba: install specific version (base env).

### 10) Favorites
- [x] Add Favorites view in sidebar navigation.
- [x] Add favorite toggle button to package rows (star icon).
- [x] Persist favorites to config file.
- [x] Filter and display favorite packages in dedicated view.
- [x] Update favorites count in sidebar.

## Implementation Phases
- **Phase A (Quick wins):** Sources UX + keyboard shortcuts + small visual polish.
- **Phase B:** List row live updates + hover affordances + consolidated feedback.
- **Phase C:** Discover tabs + inline install + caching.
- **Phase D:** Command Center + deeper onboarding + advanced shortcuts.
