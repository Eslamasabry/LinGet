# TUI Refactoring and UI/UX Improvement Plan

## Objective
Majorly improve the UI and UX of the LinGet TUI by modernizing the layout, increasing information density, improving discoverability, and refactoring the underlying architecture to be more modular and maintainable.

## Phase 1: Architectural Foundation (Componentization)
The current TUI is a monolithic `App` struct and a massive `ui.rs` file. This phase focuses on breaking it down.
- [x] Define a standard `Component` trait in `src/cli/tui/components/mod.rs` (e.g., `fn draw(&mut self, frame: &mut Frame, area: Rect)`, `fn handle_event(&mut self, event: &Event) -> Result<Option<Action>>`).
- [ ] Refactor `App` state into smaller, focused managers (e.g., `ViewState`, `SelectionState`, `QueueState`).
- [ ] Extract existing UI drawing logic from `ui.rs` into individual component modules (e.g., `SidebarComponent`, `PackageListComponent`, `DetailsComponent`).

## Phase 2: Layout & Visual Modernization (UI)
Introduce a more flexible and information-dense layout.
- [ ] **Dynamic View System:** Implement a way to switch between full-screen views (e.g., `BrowseView`, `DashboardView`, `QueueView`) rather than a single static layout.
- [x] **Multi-Column Package Table:** Replace the simple `List` widget in the packages panel with a `Table` widget.
    - Columns: Name, Version, Source, Status.
    - Implement column widths and basic styling.
- [x] **Tabbed Details Panel:** Redesign the package details view to use ratatui `Tabs`.
    - Tabs: `[Info]`, `[Dependencies]`, `[Changelog]`.
    - Render different content based on the active tab.
- [x] **Collapsible Sidebar:** Add a toggle (e.g., `Ctrl+b` or similar) to hide/show the Sources sidebar to maximize space.

## Phase 3: Interaction & Discoverability (UX)
Make the application easier to use and understand.
- [x] **Contextual Command Bar:** Implement a bottom bar (like Zellij/Helix) that shows relevant keyboard shortcuts based on the currently focused panel.
- [ ] **Interactive Task Queue:** Enhance the queue UI to allow expanding/collapsing individual tasks to see more details or logs.
- [ ] **Fuzzy Search & Highlighting:** (Optional/Stretch) Improve the search experience with visual highlighting of matched terms.
- [ ] **Mouse Support:** Ensure all new UI elements (Tabs, Table Headers) respond correctly to mouse clicks and scrolling.

## Execution Strategy
I will execute this plan sequentially, starting with Phase 1. I will commit changes after significant milestones to ensure the application remains in a working state.
