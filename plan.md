# Full Mouse Support Plan (LinGet TUI)

## 1. Goal
Deliver complete and reliable mouse support across the TUI so users can operate LinGet without keyboard dependency, while keeping keyboard-first flows unchanged.

## 2. Scope
- Enable mouse in normal TUI mode (not shell fallback mode).
- Support click, double-click, drag select (where applicable), scroll wheel, and context-sensitive actions.
- Cover all interactive regions:
  - Header filter tabs
  - Sources panel
  - Packages table
  - Details panel links/actions (if actionable)
  - Queue bar
  - Expanded queue list + logs area
  - Footer action hints (if clickable affordances are rendered)
  - Overlays (help/confirm)

## 3. Non-Goals (for this iteration)
- Terminal-specific gestures beyond standard crossterm mouse events.
- Pixel-perfect hover effects (terminal cells only).
- OS clipboard integration from mouse selection.

## 4. UX Behavior Spec

### 4.1 Global
- Left click:
  - Focus region under cursor.
  - Execute primary action for that cell/row/button.
- Double left click on package row:
  - Trigger default action:
    - `UpdateAvailable` -> queue update confirm
    - `NotInstalled` -> queue install confirm
    - `Installed` -> queue remove confirm
- Right click:
  - Open lightweight inline context strip (row-local action hints) OR immediate fallback status message if full context menu is not available.
- Mouse wheel:
  - Scroll active region under cursor.
  - If cursor is not over scrollable region, scroll focused region.

### 4.2 Header
- Click filter tab `All/Installed/Updates` applies corresponding filter.
- Click search/status area when search inactive:
  - Enter search mode and place cursor at end.

### 4.3 Sources Panel
- Click source row selects source and focuses sources panel.
- Wheel scroll moves visible window and selection accordingly.
- Click scrollbar thumb/track (if implemented in ratatui constraints):
  - Jump/drag behavior if feasible; otherwise track click jumps proportionally.

### 4.4 Packages Panel
- Click row:
  - Move cursor to row and focus packages.
- Click selection marker column:
  - Toggle row selection only.
- Double click row:
  - Trigger row default action (see 4.1).
- Wheel scroll:
  - Move package cursor up/down by configurable step (default 3 lines per wheel notch).

### 4.5 Queue Bar + Expanded Queue
- Click queue bar:
  - Toggle expanded queue.
- Expanded queue:
  - Click task row selects task.
  - Click cancel/retry hint text region triggers action when valid.
  - Wheel over task list scrolls task cursor.
  - Wheel over log pane scrolls logs.

### 4.6 Overlays
- Help overlay:
  - Click outside closes (optional) or keeps modal strict depending on final policy.
  - Click close hint closes.
- Confirm dialog:
  - Click `yes` submits.
  - Click `cancel` aborts.
  - Click outside = no-op (strict confirm safety).

## 5. Architecture & Code Plan

## 5.1 Input Pipeline (app.rs)
- Enable/disable mouse capture in lifecycle:
  - Enter alternate screen: enable mouse capture.
  - Exit/panic cleanup: disable mouse capture.
- Extend event loop to handle `Event::Mouse` and route to `App::handle_mouse(...)`.

## 5.2 App Event API (app.rs)
Add methods:
- `handle_mouse(event: MouseEvent, terminal_size: Rect) -> impl Future<Output = ()>`
- Region-specific handlers:
  - `handle_mouse_header(...)`
  - `handle_mouse_sources(...)`
  - `handle_mouse_packages(...)`
  - `handle_mouse_queue_bar(...)`
  - `handle_mouse_expanded_queue(...)`
  - `handle_mouse_overlay(...)`
- Utility methods:
  - `package_index_from_row(y)`
  - `source_index_from_row(y)`
  - `task_index_from_row(y)`
  - wheel step normalization

## 5.3 Layout Hit-Testing Contract (ui.rs + app.rs)
- Centralize geometry math in shared helpers (single source of truth).
- Return a `HitRegion` enum from a layout function:
  - `HeaderFilterTab(Filter)`
  - `HeaderSearch`
  - `SourcesRow(usize)`
  - `PackagesRow(usize)`
  - `PackagesSelectCell(usize)`
  - `QueueBar`
  - `QueueTaskRow(usize)`
  - `QueueLogArea`
  - `FooterHint(FooterAction)`
  - `Overlay(OverlayRegion)`
  - `None`
- Ensure compact/full layouts both map correctly.

## 5.4 Selection & Action Semantics
- Preserve existing keyboard semantics and reuse existing methods:
  - `set_source_by_index`, `toggle_selection_on_cursor`, `prepare_action`, `toggle_queue_expanded`, `cancel_selected_task`, `retry_selected_task`.
- Avoid divergent code paths by translating mouse interactions into the same internal actions.

## 5.5 Visual Feedback
- Update status line on mouse actions for discoverability:
  - Example: `Mouse: selected source Snap`.
- Optional transient highlight for clicked row (can reuse cursor state).

## 6. Delivery Phases

### Phase 1: Infrastructure
- Mouse capture lifecycle.
- Mouse event plumbing into app loop.
- Baseline no-op handler with logging guard.

### Phase 2: Core Panels
- Sources click + wheel.
- Packages click/select/wheel.
- Header filter tab click.

### Phase 3: Queue + Overlays
- Queue bar toggle click.
- Expanded queue row selection/cancel/retry/log scrolling.
- Confirm/help overlay clicks.

### Phase 4: Advanced Interactions
- Double click default action.
- Right-click fallback behavior.
- Optional track/thumb scrollbar behavior.

### Phase 5: Polish
- Status feedback text.
- Cursor/focus consistency audits.
- Edge-case fixes for compact mode.

## 7. Testing Plan

## 7.1 Unit Tests (app.rs/ui.rs)
- Hit-testing tests for:
  - Full layout
  - Compact layout
  - Queue expanded/collapsed
  - Overlay active/inactive
- Mouse event mapping tests:
  - Click filter tab sets expected filter.
  - Click package select marker toggles selection only.
  - Wheel in logs updates `task_log_scroll` only.

## 7.2 Integration-like Event Tests
- Simulate sequences:
  - Click package row -> double click -> confirm yes click.
  - Click queue bar -> select failed task -> click retry.

## 7.3 Regression Tests
- Ensure keyboard behavior unchanged when no mouse events occur.
- Ensure mouse input ignored in search raw text entry when appropriate.

## 7.4 Manual Validation Matrix
- Terminal emulators:
  - GNOME Terminal
  - Kitty
  - Alacritty
  - WezTerm
- Sizes:
  - Compact threshold below `COMPACT_WIDTH/COMPACT_HEIGHT`
  - Standard size
- States:
  - Loading
  - Empty filters
  - Queue active
  - Queue completed with failures

## 8. Risks & Mitigations
- Risk: layout drift between render and hit-testing.
  - Mitigation: shared geometry helper functions and tests per layout mode.
- Risk: accidental breakage of keyboard workflows.
  - Mitigation: keep mouse handlers as wrappers around existing command methods.
- Risk: terminal-specific mouse event inconsistency.
  - Mitigation: normalize event kinds and add fallback behavior for unsupported variants.

## 9. Acceptance Criteria
- User can fully operate TUI with mouse only for all primary flows:
  - filter, source selection, package selection, queue operations, confirm/cancel.
- Scroll wheel works in sources/packages/queue/logs.
- No keyboard regression in existing test suite.
- `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` pass.

## 10. Rollout
- Merge behind default-on behavior.
- If terminal incompatibility found, add config flag `tui.mouse = auto|on|off` as follow-up.
