# PRD: LinGet TUI Facelift

## Summary
Refresh the LinGet TUI to feel more intentional and readable without changing core behavior. The facelift focuses on layout, visual hierarchy, and clearer information density for package browsing, filtering, and actions.

## Goals
- Improve visual hierarchy: title, panels, table headers, and status should read clearly at a glance.
- Reduce cognitive load: consistent key hints, clearer selection state, and better empty/loading messaging.
- Make package details visible without extra keystrokes.
- Keep behavior and navigation intact (no workflow changes).

## Non-Goals
- No new package management features or backend changes.
- No new persistent config or settings.
- No dependency on external services.

## Users
- Developers and power users managing packages across multiple sources via terminal.

## Scope
In scope:
- TUI layout and styling in `src/cli/tui`.
- Visual tweaks to panels, tables, status, search, and confirm popups.

Out of scope:
- GTK/Tauri UI changes.
- Backend logic changes.

## Quality Gates
These commands must pass for every user story:
- `cargo fmt --check` - Format check
- `cargo clippy -- -D warnings` - Lint (CI enforces zero warnings)
- `cargo test` - Run all tests

For UI stories, also include:
- Run the TUI locally: `cargo run -- tui`

## User Stories

### US-001: Introduce a cohesive visual theme and layout rhythm
**Description:** As a user, I want the TUI to feel cohesive and intentional, with consistent borders, emphasis, and spacing. The layout should align panel titles, section spacing, and table header styling for quick scanning.

**Acceptance Criteria:**
- [ ] Define a small set of reusable styles (title, panel, dim text, accent, selection).
- [ ] Apply the styles consistently to title bar, panels, and table header.
- [ ] Improve panel separation and padding to make the layout feel less cramped.

### Impact Table
| Path | Change | Purpose | Notes |
|---|---|---|---|
| src/cli/tui/ui.rs | modify | Centralize styles and apply consistent layout styling | |
| src/cli/tui/mod.rs | modify | Expose new theme helpers if split into module | Optional: new module re-export |
| src/cli/tui/theme.rs | create | Reusable TUI styles and color palette | Optional file if needed |

### US-002: Add a package details panel for the current selection
**Description:** As a user, I want to see key details of the selected package (description, versions, status) without pressing Enter. This panel should update as I move the selection.

**Acceptance Criteria:**
- [ ] Details panel shows package name, installed version, available version (if any), source, and description.
- [ ] When no package is selected, the panel shows a helpful empty state message.
- [ ] Details panel respects loading and empty states without crashing.

### Impact Table
| Path | Change | Purpose | Notes |
|---|---|---|---|
| src/cli/tui/ui.rs | modify | Add details panel layout and rendering | |
| src/cli/tui/app.rs | modify | Provide any helper methods for selected package fields | Optional (if needed) |

### US-003: Refresh search and confirm popups for clarity
**Description:** As a user, I want search and confirm popups to be visually distinct and readable. The prompt should clearly indicate what action is being confirmed or what text I am entering.

**Acceptance Criteria:**
- [ ] Search popup shows a labeled input field with cursor indicator.
- [ ] Confirm popup clearly shows the action (install/remove) and the package name.
- [ ] Popups include consistent borders and contrast against the background.

### Impact Table
| Path | Change | Purpose | Notes |
|---|---|---|---|
| src/cli/tui/ui.rs | modify | Redesign popup layout and styles | |
| src/cli/tui/app.rs | modify | Improve confirm message text formatting | |

### US-004: Improve status and command bars for quick scanning
**Description:** As a user, I want the status and command bars to be easier to parse. Key hints should be grouped, and status should be concise and readable.

**Acceptance Criteria:**
- [ ] Command bar groups related actions (navigation, filters, actions, quit).
- [ ] Status bar highlights mode and shows concise status text with optional loading indicator.
- [ ] Status remains readable when messages are long (truncate or wrap intentionally).

### Impact Table
| Path | Change | Purpose | Notes |
|---|---|---|---|
| src/cli/tui/ui.rs | modify | Redesign status and command bars | |
| src/cli/tui/app.rs | modify | Optional helpers for status text formatting | |
