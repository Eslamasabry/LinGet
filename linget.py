#!/usr/bin/env python3
"""
LinGet - The Universal Package Manager
A rich, perfect, and fancy TUI for managing system packages.
"""

from textual.app import App, ComposeResult
from textual.screen import ModalScreen
from textual.widgets import (
    DataTable,
    Static,
    Footer,
    Header,
    Input,
    Button,
    Label,
    ProgressBar,
    LoadingIndicator,
    TabbedContent,
    TabPane,
    RichLog,
    Markdown,
    Tabs,
    Tab,
)
from textual.containers import Horizontal, Vertical, VerticalScroll, Container
from textual.reactive import reactive
from textual.binding import Binding
from textual import work
from textual.css.query import NoMatches
from textual.timer import Timer
from textual.command import CommandPalette

import asyncio
import re
import json
import os
import sys
import shutil
import time
import urllib.error
from typing import List, Optional, Set, Dict, Any, Tuple
from pathlib import Path

# Import modular components
from linget.models import (
    Package,
    Task,
    PackageStatus,
    ErrorType,
    load_favorites,
    save_favorites,
    is_favorite,
)
from linget.search import search_new_packages
from linget.history import save_task, load_task_history
from linget.settings import load_settings, save_settings
from linget.plugins import get_plugin_registry, load_plugins, PluginRegistry
from linget.cache import (
    clear_cache,
    load_cached_packages,
    save_cached_packages,
    is_cache_valid,
    get_cache_timestamp,
    get_cache_age_text,
    should_use_cache,
)
from linget import logger, validation

# Setup logging
logger.setup_logging(level=logger.logging.INFO, log_to_console=False)

# Constants
MAX_CONCURRENT_TASKS = 50
DEFAULT_TIMEOUT = 300


# --- Custom Widgets ---


class PackageTable(DataTable):
    """Custom DataTable widget for displaying package information with bulk selection support."""

    # Fixed column widths: ☐, Status, Source
    _COL_CHECK_W = 3
    _COL_STATUS_W = 12
    _COL_SOURCE_W = 10
    _FIXED_TOTAL = _COL_CHECK_W + _COL_STATUS_W + _COL_SOURCE_W

    def on_mount(self):
        """Initialize the table with columns and styling."""
        self.cursor_type = "row"
        self._ck_check = self.add_column("☐", width=self._COL_CHECK_W, key="check")
        self._ck_status = self.add_column("Status", width=self._COL_STATUS_W, key="status")
        self._ck_name = self.add_column("Name", key="name")
        self._ck_version = self.add_column("Version", key="version")
        self._ck_source = self.add_column("Source", width=self._COL_SOURCE_W, key="source")
        self.zebra_stripes = True
        self.selected_rows: Set[str] = set()
        self._fit_columns()

    def on_resize(self, event) -> None:
        """Recalculate column widths when the table is resized."""
        self._fit_columns()

    def _fit_columns(self) -> None:
        """Set Name and Version column widths to fill available space."""
        # Reserve 1 col for vertical scrollbar (CSS scrollbar-size: 1 1)
        padding = self.cell_padding * 2 * 5  # 5 columns, each with left+right padding
        avail = self.size.width - self._FIXED_TOTAL - padding - 1
        if avail < 20:
            return
        name_w = max(10, int(avail * 0.55))
        version_w = max(8, avail - name_w)
        for ck, width in [(self._ck_name, name_w), (self._ck_version, version_w)]:
            col = self.columns.get(ck)
            if col is not None:
                col.auto_width = False
                col.width = width
        self.refresh()

    def populate(
        self, packages: List[Package], favorites: Optional[Set[str]] = None
    ) -> None:
        """Populate the table with package data.

        Args:
            packages: List of Package objects to display
            favorites: Optional set of favorited package row_keys
        """
        self.clear()
        favorites = favorites or set()

        # Track added keys to prevent duplicates
        added_keys: Set[str] = set()

        for pkg in packages:
            row_key = f"{pkg.source}-{pkg.name}"

            # Skip duplicates
            if row_key in added_keys:
                continue
            added_keys.add(row_key)

            status_render = {
                PackageStatus.INSTALLED: "[green]● Installed[/]",
                PackageStatus.UPDATE: "[yellow bold]◆ Update[/]",
                PackageStatus.NOT_INSTALLED: "[dim]○ Available[/]",
            }.get(pkg.status, "[dim]? Unknown[/]")

            source_color = {
                "apt": "red",
                "flatpak": "blue",
                "cargo": "yellow",
                "npm": "green",
                "pip": "cyan",
                "snap": "magenta",
                "aur": "cyan",
                "dnf": "blue",
                "brew": "orange",
            }.get(pkg.source, "white")

            source_logo = {
                "apt": " APT",
                "flatpak": "󰏖 Flatpak",
                "cargo": " Cargo",
                "npm": " NPM",
                "pip": " PIP",
                "snap": "📦 Snap",
                "aur": "🗼 AUR",
                "dnf": "🎩 DNF",
                "brew": "🍺 Brew",
            }.get(pkg.source, pkg.source.upper())

            checkbox = "☑" if row_key in self.selected_rows else "☐"

            try:
                self.add_row(
                    checkbox,
                    status_render,
                    f"[bold]{pkg.name}[/]",
                    pkg.version,
                    f"[bold {source_color}]{source_logo}[/]",
                    key=row_key,
                )
            except (KeyError, ValueError, TypeError) as e:
                logger.error(f"Failed to add row for {pkg.name}: {e}")

        self._fit_columns()


class InfoPanel(VerticalScroll):
    """Panel for displaying detailed package information."""

    package = reactive(None)

    def render_info(self, favorites: Optional[Set[str]] = None) -> str:
        """Render package information as markdown.

        Args:
            favorites: Optional set of favorited package row_keys

        Returns:
            Markdown-formatted string with package details
        """
        if not self.package:
            return "[dim italic]Select a package to view details...[/]"

        p = self.package
        favorites = favorites or set()
        is_fav = p.row_key in favorites
        fav_icon = "⭐ " if is_fav else ""

        status_text = {
            PackageStatus.INSTALLED: "✅ Installed",
            PackageStatus.UPDATE: "🔄 Update Available",
            PackageStatus.NOT_INSTALLED: "📥 Not Installed",
        }.get(p.status, "Unknown")

        source_logo = {
            "apt": " APT",
            "flatpak": "󰏖 Flatpak",
            "cargo": " Cargo",
            "npm": " NPM",
            "pip": " PIP",
            "snap": "📦 Snap",
            "aur": "🗼 AUR",
            "dnf": "🎩 DNF",
            "brew": "🍺 Brew",
        }.get(p.source, p.source.upper())

        return f"""# {fav_icon}{p.name}

| Field | Value |
|-------|-------|
| Version | `{p.version}` |
| Source | {source_logo} |
| Size | {p.size or "Unknown"} |
| Status | **{status_text}** |

{p.description or "_No description provided by the package manager._"}

---
**Actions:** `i` Install · `u` Update · `r` Remove · `f` Favorite
"""

    def watch_package(self, package: Optional[Package]):
        """React to package selection changes.

        Args:
            package: The newly selected package or None
        """
        for child in list(self.children):
            child.remove()

        if not package:
            self.mount(
                Static(
                    "[dim italic]Select a package to view details...[/]",
                    classes="empty-info",
                )
            )
            return

        favorites = getattr(self.app, "favorites", set())
        self.mount(Markdown(self.render_info(favorites=favorites)))


class PackageDetailScreen(ModalScreen):
    """Modal screen for displaying package details."""

    DEFAULT_CSS = """
    PackageDetailScreen {
        align: center middle;
    }
    PackageDetailScreen > #detail-container {
        width: 70;
        max-width: 90%;
        height: auto;
        max-height: 80%;
        background: $surface;
        border: round $accent;
        padding: 1 2;
    }
    PackageDetailScreen > #detail-container Markdown {
        margin: 0;
    }
    PackageDetailScreen > #detail-container #detail-actions {
        height: 3;
        align-horizontal: center;
        margin-top: 1;
    }
    PackageDetailScreen > #detail-container #detail-actions Button {
        margin: 0 1;
    }
    """

    BINDINGS = [
        Binding("escape", "dismiss", "Close"),
        Binding("i", "do_install", "Install"),
        Binding("u", "do_update", "Update"),
        Binding("r", "do_remove", "Remove"),
        Binding("f", "do_favorite", "Favorite"),
    ]

    def __init__(self, package: Package, favorites: Set[str], **kwargs):
        super().__init__(**kwargs)
        self._package = package
        self._favorites = favorites

    def compose(self) -> ComposeResult:
        p = self._package
        is_fav = p.row_key in self._favorites
        fav_icon = "⭐ " if is_fav else ""

        status_text = {
            PackageStatus.INSTALLED: "✅ Installed",
            PackageStatus.UPDATE: "🔄 Update Available",
            PackageStatus.NOT_INSTALLED: "📥 Not Installed",
        }.get(p.status, "Unknown")

        source_logo = {
            "apt": " APT", "flatpak": "󰏖 Flatpak",
            "cargo": " Cargo", "npm": " NPM", "pip": " PIP",
            "snap": "📦 Snap", "aur": "🗼 AUR",
            "dnf": "🎩 DNF", "brew": "🍺 Brew",
        }.get(p.source, p.source.upper())

        md = f"""# {fav_icon}{p.name}

| Field | Value |
|-------|-------|
| Version | `{p.version}` |
| Source | {source_logo} |
| Status | **{status_text}** |

{p.description or "_No description provided._"}
"""
        with Vertical(id="detail-container"):
            yield Markdown(md)
            with Horizontal(id="detail-actions"):
                yield Button("Install [i]", id="btn-install", variant="success")
                yield Button("Update [u]", id="btn-update", variant="primary")
                yield Button("Remove [r]", id="btn-remove", variant="error")
                fav_label = "★ Unfav [f]" if is_fav else "☆ Fav [f]"
                yield Button(fav_label, id="btn-favorite", variant="default")

    def on_button_pressed(self, event: Button.Pressed) -> None:
        actions = {
            "btn-install": "install",
            "btn-update": "update",
            "btn-remove": "remove",
            "btn-favorite": "favorite",
        }
        action = actions.get(event.button.id)
        if action:
            self.dismiss(action)

    def action_do_install(self):
        self.dismiss("install")

    def action_do_update(self):
        self.dismiss("update")

    def action_do_remove(self):
        self.dismiss("remove")

    def action_do_favorite(self):
        self.dismiss("favorite")


class SudoPasswordScreen(ModalScreen[Optional[str]]):
    """Modal screen for collecting a sudo password."""

    DEFAULT_CSS = """
    SudoPasswordScreen {
        align: center middle;
    }
    SudoPasswordScreen > #sudo-dialog {
        width: 60;
        max-width: 90%;
        height: auto;
        background: $surface;
        border: round $warning;
        padding: 1 2;
    }
    SudoPasswordScreen > #sudo-dialog Label {
        margin-bottom: 1;
    }
    SudoPasswordScreen > #sudo-dialog Input {
        margin-bottom: 1;
    }
    SudoPasswordScreen > #sudo-dialog #sudo-actions {
        height: 3;
        align-horizontal: right;
    }
    SudoPasswordScreen > #sudo-dialog #sudo-actions Button {
        margin-left: 1;
    }
    """

    BINDINGS = [Binding("escape", "cancel", "Cancel")]

    def compose(self) -> ComposeResult:
        with Vertical(id="sudo-dialog"):
            yield Label("Administrative privileges required. Enter your sudo password:")
            yield Input(
                placeholder="Password",
                password=True,
                id="sudo-password",
            )
            with Horizontal(id="sudo-actions"):
                yield Button("Cancel", id="sudo-cancel")
                yield Button("Unlock", id="sudo-submit", variant="primary")

    def on_mount(self) -> None:
        self.query_one("#sudo-password", Input).focus()

    def on_input_submitted(self, event: Input.Submitted) -> None:
        if event.input.id == "sudo-password":
            self._submit()

    def on_button_pressed(self, event: Button.Pressed) -> None:
        if event.button.id == "sudo-submit":
            self._submit()
        elif event.button.id == "sudo-cancel":
            self.dismiss(None)

    def _submit(self) -> None:
        password = self.query_one("#sudo-password", Input).value
        if not password:
            self.app.notify("Password is required", severity="warning")
            return
        self.dismiss(password)

    def action_cancel(self) -> None:
        self.dismiss(None)


class TaskRow(Horizontal):
    """Widget for displaying a task in the queue with progress."""

    def __init__(self, task: Task, **kwargs):
        """Initialize task row with task data.

        Args:
            task: Task object containing package and action info
            **kwargs: Additional widget arguments
        """
        super().__init__(**kwargs)
        self.task_data = task
        self.progress_bar = ProgressBar(total=100, show_eta=False)

    def compose(self) -> ComposeResult:
        """Compose the task row UI."""
        icon = {"install": "⬇", "update": "⬆", "remove": "✖"}.get(
            self.task_data.action, "▶"
        )
        color = {"install": "green", "update": "yellow", "remove": "red"}.get(
            self.task_data.action, "blue"
        )

        yield Label(
            f"[{color} bold]{icon}[/] {self.task_data.package.name}",
            classes="task-label",
        )
        yield self.progress_bar
        yield Label(
            "[dim]◌ Queued[/]", id=f"status-{self.task_data.dom_id}", classes="task-status"
        )

    def update_progress(self, progress: float, status: str) -> None:
        """Update the task progress display.

        Args:
            progress: Progress percentage (0-100)
            status: Current status string (running, done, error)
        """
        self.progress_bar.progress = progress
        try:
            status_label = self.query_one(f"#status-{self.task_data.dom_id}", Label)
            if status == "running":
                status_label.update("[cyan bold]● Running[/]")
            elif status == "done":
                status_label.update("[green bold]✓ Complete[/]")
            elif status == "cancelled":
                status_label.update("[yellow]⊘ Cancelled[/]")
            elif status == "error":
                status_label.update("[red bold]✗ Failed[/]")
        except Exception:
            pass


class QueuePanel(VerticalScroll):
    """Panel for displaying the task queue."""

    def compose(self) -> ComposeResult:
        """Compose the queue panel UI."""
        yield Label("⏳ No active tasks", id="empty-queue")

    def add_task(self, task: Task) -> TaskRow:
        """Add a task to the queue panel.

        Args:
            task: Task object to add

        Returns:
            The created TaskRow widget
        """
        empty_label = self.query("#empty-queue")
        if empty_label:
            empty_label.remove()

        row = TaskRow(task, id=f"task-row-{task.dom_id}")
        self.mount(row)
        self.scroll_end(animate=True)
        return row


# --- Command Palette ---


class LingetCommandPalette(CommandPalette):
    """Custom command palette for LinGet with all app actions."""

    def on_mount(self):
        """Register all LinGet commands when palette mounts."""
        self.add_command(
            "Install selected package",
            self.action_install,
            tooltip="Install the currently selected package (i)",
        )
        self.add_command(
            "Update selected package",
            self.action_update,
            tooltip="Update the currently selected package (u)",
        )
        self.add_command(
            "Remove selected package",
            self.action_remove,
            tooltip="Remove the currently selected package (r)",
        )
        self.add_command(
            "Toggle select package",
            self.action_toggle_select,
            tooltip="Toggle selection for bulk operations (Space)",
        )
        self.add_command(
            "Select all packages",
            self.action_select_all,
            tooltip="Select all visible packages (a)",
        )
        self.add_command(
            "Deselect all packages",
            self.action_deselect_all,
            tooltip="Clear all selections (A)",
        )
        self.add_command(
            "Bulk install selected",
            self.action_bulk_install,
            tooltip="Install all selected packages (I)",
        )
        self.add_command(
            "Bulk update selected",
            self.action_bulk_update,
            tooltip="Update all selected packages (U)",
        )
        self.add_command(
            "Focus search",
            self.action_focus_search,
            tooltip="Focus the search input (/)",
        )
        self.add_command(
            "Change to All Sources",
            self._set_source_all,
            tooltip="View all package sources",
        )
        self.add_command(
            "Change to Favorites",
            self._set_source_favorites,
            tooltip="View favorite packages",
        )
        self.add_command(
            "Change to APT",
            self._set_source_apt,
            tooltip="View APT packages",
        )
        self.add_command(
            "Change to Flatpak",
            self._set_source_flatpak,
            tooltip="View Flatpak applications",
        )
        self.add_command(
            "View All Packages mode",
            self._set_mode_all,
            tooltip="Show all packages",
        )
        self.add_command(
            "View Updates mode",
            self._set_mode_updates,
            tooltip="Show only packages with updates",
        )
        self.add_command(
            "Search for New mode",
            self._set_mode_search,
            tooltip="Search for new packages to install",
        )
        self.add_command(
            "Show dependencies",
            self.action_show_dependencies,
            tooltip="Show package dependencies (d)",
        )
        self.add_command(
            "Show version history",
            self.action_show_versions,
            tooltip="Show available versions for package (v)",
        )
        self.add_command(
            "Show orphan packages",
            self.action_show_orphans,
            tooltip="Find orphaned packages that can be removed (o)",
        )
        self.add_command(
            "Toggle favorite",
            self.action_toggle_favorite,
            tooltip="Add/remove current package from favorites (f)",
        )
        self.add_command(
            "Clean cache",
            self.action_clean_cache,
            tooltip="Clean package manager cache (X)",
        )
        self.add_command(
            "Refresh package list",
            self.action_refresh_data,
            tooltip="Refresh all package data (Ctrl+r)",
        )
        self.add_command(
            "Clear completed tasks",
            self.action_clear_tasks,
            tooltip="Remove finished tasks from queue (c)",
        )
        self.add_command(
            "Cancel running task",
            self.action_cancel_task,
            tooltip="Cancel the currently running task (Escape)",
        )
        self.add_command(
            "Retry failed task",
            self.action_retry_task,
            tooltip="Retry the last failed task (R)",
        )
        self.add_command(
            "Undo last action",
            self.action_undo,
            tooltip="Undo the last package operation (z)",
        )
        self.add_command(
            "Quit LinGet",
            self.action_quit,
            tooltip="Exit the application (q)",
        )

    def _set_source_all(self):
        self._set_source("all")

    def _set_source_favorites(self):
        self._set_source("favorites")

    def _set_source_apt(self):
        self._set_source("apt")

    def _set_source_flatpak(self):
        self._set_source("flatpak")

    def _set_source(self, source_id: str):
        """Set the current package source filter.

        Args:
            source_id: Source identifier string
        """
        app = self.app
        app.current_source = source_id
        app.apply_filters()
        self.dismiss()

    def _set_mode_all(self):
        self._set_mode("mode-all")

    def _set_mode_updates(self):
        self._set_mode("mode-updates")

    def _set_mode_search(self):
        self._set_mode("mode-search")

    def _set_mode(self, mode_id: str):
        """Set the current view mode.

        Args:
            mode_id: Mode identifier string
        """
        app = self.app
        app.current_mode = mode_id
        app.apply_filters()
        self.dismiss()

    def action_install(self):
        self.app.action_install()
        self.dismiss()

    def action_update(self):
        self.app.action_update()
        self.dismiss()

    def action_remove(self):
        self.app.action_remove()
        self.dismiss()

    def action_toggle_select(self):
        self.app.action_toggle_select()
        self.dismiss()

    def action_select_all(self):
        self.app.action_select_all()
        self.dismiss()

    def action_deselect_all(self):
        self.app.action_deselect_all()
        self.dismiss()

    def action_bulk_install(self):
        self.app.action_bulk_install()
        self.dismiss()

    def action_bulk_update(self):
        self.app.action_bulk_update()
        self.dismiss()

    def action_focus_search(self):
        self.app.action_focus_search()
        self.dismiss()

    def action_show_dependencies(self):
        self.app.run_worker(self.app.action_show_dependencies(), exclusive=False)
        self.dismiss()

    def action_show_versions(self):
        self.app.run_worker(self.app.action_show_versions(), exclusive=False)
        self.dismiss()

    def action_show_orphans(self):
        self.app.run_worker(self.app.action_show_orphans(), exclusive=False)
        self.dismiss()

    def action_toggle_favorite(self):
        self.app.action_toggle_favorite()
        self.dismiss()

    def action_clean_cache(self):
        self.app.run_worker(self.app.action_clean_cache(), exclusive=False)
        self.dismiss()

    def action_refresh_data(self):
        self.app.action_refresh_data()
        self.dismiss()

    def action_clear_tasks(self):
        self.app.action_clear_tasks()
        self.dismiss()

    def action_cancel_task(self):
        self.app.action_cancel_task()
        self.dismiss()

    def action_retry_task(self):
        self.app.action_retry_task()
        self.dismiss()

    def action_undo(self):
        self.app.action_undo()
        self.dismiss()

    def action_quit(self):
        self.app.action_quit()


# --- Main Application ---


class LinGetApp(App):
    """A rich, elegant TUI for package management."""

    CSS = """
    Screen {
        background: $background;
        overflow: hidden hidden;
    }

    /* ── Main Layout ───────────────────────────── */
    #main-layout {
        height: 1fr;
    }
    #content-area {
        height: 1fr;
        width: 1fr;
    }

    /* ── Mode Tabs ─────────────────────────────── */
    #mode-tabs {
        dock: top;
        height: 2;
        background: $surface;
    }

    /* ── Source Tabs ───────────────────────────── */
    #source-tabs {
        dock: top;
        height: 2;
        background: $surface-darken-1;
        scrollbar-size: 0 0;
    }
    #source-tabs Underline {
        height: 0;
    }
    #source-tabs .tab {
        padding: 0 1;
        min-width: 6;
    }
    #source-tabs .tab.-active {
        text-style: bold;
        color: $accent;
    }

    /* ── Toolbar ───────────────────────────────── */
    #toolbar {
        height: 3;
        padding: 0 1;
        background: $surface;
        align-vertical: middle;
    }
    #search {
        width: 1fr;
        margin-right: 1;
        border: tall $panel;
    }
    #search:hover {
        border: tall $accent 50%;
    }
    #search:focus {
        border: tall $accent;
    }
    #toolbar.compact #search {
        width: 100%;
        margin-right: 0;
    }
    #refresh-btn {
        min-width: 14;
        margin: 0;
    }
    #toolbar.compact #refresh-btn {
        display: none;
    }

    /* ── Package Table ─────────────────────────── */
    PackageTable {
        height: 1fr;
        border: none;
        scrollbar-size: 1 1;
    }
    PackageTable > .datatable--header {
        background: $surface;
        text-style: bold;
        color: $text;
    }
    PackageTable > .datatable--cursor {
        background: $accent 25%;
        text-style: bold;
    }
    PackageTable > .datatable--hover {
        background: $boost;
    }

    /* ── Bottom Panel ──────────────────────────── */
    #bottom-panel {
        height: 12;
        dock: bottom;
        background: $surface;
        border: round $panel;
        border-title-color: $text;
        border-title-style: bold;
        border-title-align: center;
        padding: 0;
    }
    #bottom-panel.collapsed {
        height: 3;
    }
    #bottom-panel.compact {
        height: 7;
    }

    /* ── Task Queue ────────────────────────────── */
    #queue-panel {
        height: 1fr;
        padding: 0;
        scrollbar-size: 1 1;
    }
    #empty-queue {
        text-align: center;
        margin-top: 1;
        color: $text-muted;
    }

    TaskRow {
        height: 2;
        margin: 0 1;
        padding: 0 1;
        background: transparent;
    }
    .task-label {
        width: 22;
    }
    ProgressBar {
        width: 1fr;
        margin: 0 2;
    }
    .task-status {
        width: 14;
        text-align: right;
    }
    #bottom-panel.compact TaskRow { height: 1; }
    #bottom-panel.compact .task-label { width: 14; }
    #bottom-panel.compact ProgressBar { margin: 0 1; }
    #bottom-panel.compact .task-status { width: 10; }

    /* ── Terminal Log ──────────────────────────── */
    #term-log {
        height: 1fr;
        padding: 0 1;
        scrollbar-size: 1 1;
    }

    /* ── Loading Overlay ───────────────────────── */
    #loading-overlay {
        width: 100%;
        height: 100%;
        background: $background 60%;
        align: center middle;
        layer: overlay;
        display: none;
    }
    #loading-overlay.-active {
        display: block;
    }
    .loading-box {
        width: 44;
        height: auto;
        max-height: 7;
        background: $surface;
        border: round $accent;
        content-align: center middle;
        padding: 1 2;
    }
    #loading-msg {
        text-align: center;
        text-style: bold;
        color: $accent;
    }

    /* ── Misc ──────────────────────────────────── */
    TabbedContent {
        height: 1fr;
    }
    ContentSwitcher {
        height: 1fr;
    }
    TabPane {
        height: 1fr;
        padding: 0;
    }
    """

    BINDINGS = [
        Binding("q", "quit", "Quit", show=True),
        Binding("enter", "show_details", "Details", show=True),
        Binding("i", "install", "Install", show=True),
        Binding("u", "update", "Update", show=True),
        Binding("r", "remove", "Remove", show=True),
        Binding("space", "toggle_select", "Select", show=True),
        Binding("/", "focus_search", "Search", show=True),
        Binding("ctrl+r", "refresh_data", "Refresh", show=True),
        Binding("f", "toggle_favorite", "★ Fav", show=True),
        Binding("escape", "cancel_task", "Cancel", show=True),
        Binding("a", "select_all", "Select All", show=False),
        Binding("A", "deselect_all", "Deselect All", show=False),
        Binding("I", "bulk_install", "Bulk Install", show=False),
        Binding("U", "bulk_update", "Bulk Update", show=False),
        Binding("R", "retry_task", "Retry", show=False),
        Binding("d", "show_dependencies", "Deps", show=False),
        Binding("z", "undo", "Undo", show=False),
        Binding("o", "show_orphans", "Orphans", show=False),
        Binding("X", "clean_cache", "Clean Cache", show=False),
        Binding("v", "show_versions", "Versions", show=False),
        Binding("c", "clear_tasks", "Clear Queue", show=False),
        Binding("ctrl+shift+r", "force_refresh", "Force Refresh", show=False),
        Binding("ctrl+e", "export_packages", "Export", show=False),
        Binding("ctrl+i", "import_packages", "Import", show=False),
    ]

    all_packages: List[Package] = []
    tasks: List[Task] = []
    current_source: str = "all"
    current_mode: str = "mode-all"
    search_query: str = ""
    _running_tasks: Dict[str, Any] = {}
    selected_packages: Set[str] = set()
    _last_action: Optional[Tuple[Package, str]] = None
    favorites: Set[str] = set()
    _settings: Dict[str, Any] = {}
    _plugin_registry: Optional[PluginRegistry] = None
    _force_refresh: bool = False
    _cache_refresh_in_progress: bool = False
    _offline_mode: bool = False
    _is_macos: bool = False
    _has_brew: bool = False
    _auto_refresh_timer: Optional[Timer] = None
    _search_timer: Optional[Timer] = None
    _startup_log_lines: List[str] = []
    _plugin_count: int = 0
    _fetch_in_progress: bool = False
    _user_offline_mode: bool = False
    _auto_offline_mode: bool = False
    _search_request_id: int = 0
    _pending_remove: Optional[Tuple[str, float]] = None
    _pending_clean_cache: Optional[Tuple[str, float]] = None
    _selected_package: Optional[Package] = None
    _sudo_password: Optional[str] = None

    def compose(self) -> ComposeResult:
        """Compose the main application UI."""
        yield Header(show_clock=True, icon="📦")

        with Vertical(id="main-layout"):
            with Vertical(id="content-area"):
                yield Tabs(
                    Tab("📦 All", id="mode-all"),
                    Tab("⬆️ Updates", id="mode-updates"),
                    Tab("🔍 Search", id="mode-search"),
                    id="mode-tabs",
                )
                yield Tabs(
                    Tab("🌍 All", id="src-all"),
                    Tab("⭐ Fav", id="src-favorites"),
                    Tab(" APT", id="src-apt"),
                    Tab("󰏖 Flatpak", id="src-flatpak"),
                    Tab("📦 Snap", id="src-snap"),
                    Tab("🗼 AUR", id="src-aur"),
                    Tab("🎩 DNF", id="src-dnf"),
                    Tab("🍺 Brew", id="src-brew"),
                    Tab(" Cargo", id="src-cargo"),
                    Tab(" NPM", id="src-npm"),
                    Tab(" PIP", id="src-pip"),
                    id="source-tabs",
                )
                with Horizontal(id="toolbar"):
                    yield Input(
                        placeholder="🔍 Search... (/ to focus)",
                        id="search",
                    )
                    yield Button("↻ Refresh", id="refresh-btn", variant="primary")

                yield PackageTable(id="package-table")

            with Vertical(id="bottom-panel"):
                with TabbedContent():
                    with TabPane("📋 Tasks"):
                        yield QueuePanel(id="queue-panel")
                    with TabPane("💻 Terminal"):
                        yield RichLog(
                            id="term-log",
                            highlight=True,
                            markup=True,
                            wrap=True,
                            max_lines=1000,
                        )

        with Vertical(id="loading-overlay"):
            with Vertical(classes="loading-box"):
                yield LoadingIndicator()
                yield Label("Initializing...", id="loading-msg")

        yield Footer()

    def on_mount(self):
        """Initialize the application on mount."""
        self.title = "LinGet - Universal Package Manager"
        self._startup_log_lines = []

        self._settings = load_settings()
        self.theme = self._settings.get("theme", "monokai")
        self._user_offline_mode = self._settings.get("offline_mode", False)
        self._offline_mode = self._user_offline_mode
        self.current_source = self._settings.get("default_source", "all")
        self.current_mode = self._settings.get("default_mode", "mode-updates")

        self._is_macos = sys.platform == "darwin"
        self._has_brew = False
        if self._is_macos:
            self._has_brew = shutil.which("brew") is not None

        self.favorites = load_favorites()

        cached_packages = load_cached_packages()
        if cached_packages and should_use_cache():
            self.all_packages = sorted(cached_packages, key=lambda p: p.name.lower())

        self._plugin_registry = get_plugin_registry()
        self._plugin_count = load_plugins(self._plugin_registry)
        if self._plugin_count > 0:
            self._startup_log_lines.append(
                f"[cyan]Plugins:[/] Loaded {self._plugin_count} plugin(s)"
            )
            for error in self._plugin_registry.load_errors:
                self._startup_log_lines.append(f"[yellow]Plugin warning:[/] {error}")

        self.call_later(self._complete_startup)

        if self._settings.get("auto_refresh", True):
            self._start_auto_refresh_timer()

        self.set_interval(30, self._check_network)

    def _complete_startup(self) -> None:
        """Finish startup work after the screen tree is fully available."""
        try:
            bp = self.query_one("#bottom-panel")
            bp.border_title = "⚙ Tasks & Output"
            bp.add_class("collapsed")
        except NoMatches:
            pass

        if self._plugin_count > 0:
            self._add_plugin_sources_to_tabs()

        self._apply_settings_to_ui()
        self._update_responsive_layout()

        if self.all_packages:
            self.apply_filters()
            self.notify(
                f"Loaded {len(self.all_packages)} packages from cache",
                severity="information",
                timeout=2,
            )
            self._cache_refresh_in_progress = True
            asyncio.ensure_future(self._background_fetch())
        else:
            self.action_refresh_data()

        if self._startup_log_lines:
            try:
                term_log = self.query_one("#term-log", RichLog)
                for line in self._startup_log_lines:
                    term_log.write(line)
            except NoMatches:
                pass

    def _set_widget_class(self, selector: str, class_name: str, enabled: bool) -> None:
        """Toggle a CSS class on a widget if it is mounted."""
        try:
            widget = self.query_one(selector)
        except NoMatches:
            return
        if enabled:
            widget.add_class(class_name)
        else:
            widget.remove_class(class_name)

    def _update_responsive_layout(self) -> None:
        """Adjust layout classes for smaller terminal sizes."""
        width = self.size.width
        height = self.size.height
        compact = width < 85 or height < 28

        self._set_widget_class("#toolbar", "compact", compact)
        self._set_widget_class("#bottom-panel", "compact", compact)

    def on_resize(self, event) -> None:
        """Recompute responsive layout when the terminal size changes."""
        self._update_responsive_layout()

    def _expand_bottom_panel(self):
        """Expand the bottom panel to show tasks."""
        try:
            self.query_one("#bottom-panel").remove_class("collapsed")
        except NoMatches:
            pass

    def _collapse_bottom_panel(self):
        """Collapse the bottom panel when no tasks are active."""
        try:
            self.query_one("#bottom-panel").add_class("collapsed")
        except NoMatches:
            pass

    def _sync_bottom_panel_state(self) -> None:
        """Expand for active tasks and collapse when everything is finished."""
        active_statuses = {"pending", "queued", "running"}
        has_active_tasks = any(task.status in active_statuses for task in self.tasks)
        if has_active_tasks:
            self._expand_bottom_panel()
        else:
            self._collapse_bottom_panel()

    def _apply_settings_to_ui(self):
        """Apply loaded settings to UI widgets."""
        try:
            mode_tabs = self.query_one("#mode-tabs", Tabs)
            mode_tabs.active = self.current_mode
        except Exception:
            pass
        try:
            source_tabs = self.query_one("#source-tabs", Tabs)
            tab_id = f"src-{self.current_source}"
            source_tabs.active = tab_id
        except Exception:
            pass

    def _add_plugin_sources_to_tabs(self):
        """Add plugin sources to the source tabs dynamically."""
        if not self._plugin_registry:
            return
        try:
            source_tabs = self.query_one("#source-tabs", Tabs)
            for plugin in self._plugin_registry.plugins:
                source_tabs.add_tab(
                    Tab(f"🔌 {plugin.name.title()}", id=f"src-{plugin.name}")
                )
        except Exception as e:
            logger.error(f"Plugin tabs error: {e}")

    def _start_auto_refresh_timer(self) -> None:
        """Start or replace the auto-refresh timer using the saved interval."""
        self._stop_auto_refresh_timer()
        interval = self._settings.get("refresh_interval", 600)
        self._auto_refresh_timer = self.set_interval(interval, self._background_refresh)

    def _stop_auto_refresh_timer(self) -> None:
        """Stop the auto-refresh timer if it is currently running."""
        if self._auto_refresh_timer is not None:
            self._auto_refresh_timer.stop()
            self._auto_refresh_timer = None

    def _show_loading_overlay(self) -> None:
        """Show the loading overlay when it is mounted."""
        try:
            self.query_one("#loading-overlay").add_class("-active")
        except NoMatches:
            pass

    def _hide_loading_overlay(self) -> None:
        """Hide the loading overlay when it is mounted."""
        try:
            self.query_one("#loading-overlay").remove_class("-active")
        except NoMatches:
            pass

    def _update_effective_offline_mode(self) -> None:
        """Recompute effective offline mode from user and network state."""
        self._offline_mode = self._user_offline_mode or self._auto_offline_mode

    def _get_system_pip_cmd(self) -> Optional[List[str]]:
        """Return a system pip command without targeting LinGet's own venv."""
        for executable in ("pip3", "pip"):
            pip_path = shutil.which(executable)
            if pip_path and Path(pip_path).resolve() != Path(sys.executable).resolve():
                return [pip_path]
        return None

    def _pip_is_externally_managed(self) -> bool:
        """Check if the system Python is PEP 668 externally managed."""
        try:
            lib_dir = Path(sys.base_prefix) / "lib"
            for d in lib_dir.iterdir():
                marker = d / "EXTERNALLY-MANAGED"
                if marker.exists():
                    return True
        except (OSError, StopIteration):
            pass
        return False

    def _resolve_aur_helper(self) -> Optional[str]:
        """Return the preferred available AUR helper."""
        for helper in ("yay", "paru"):
            if shutil.which(helper):
                return helper
        return None

    def _build_privileged_cmd(self, *args: str) -> List[str]:
        """Prefer pkexec in GUI sessions, otherwise fall back to terminal sudo."""
        has_graphical_auth = bool(
            os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY")
        )
        if has_graphical_auth and shutil.which("pkexec"):
            return ["pkexec", *args]
        if shutil.which("sudo"):
            return ["sudo", "-S", "-p", "", *args]
        return ["pkexec", *args]

    async def _prompt_sudo_password(self) -> Optional[str]:
        """Prompt for a sudo password inside the TUI."""
        return await self.push_screen_wait(SudoPasswordScreen())

    async def _validate_sudo_password(self, password: str) -> bool:
        """Validate sudo credentials without running the package command."""
        try:
            proc = await asyncio.create_subprocess_exec(
                "sudo",
                "-S",
                "-p",
                "",
                "-v",
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.STDOUT,
            )
            proc.stdin.write(f"{password}\n".encode())
            await proc.stdin.drain()
            proc.stdin.close()
            await proc.communicate()
            return proc.returncode == 0
        except (OSError, ValueError):
            return False

    async def _ensure_sudo_password(self) -> bool:
        """Ensure we have valid sudo credentials cached for this TUI session."""
        if not shutil.which("sudo"):
            return False

        attempts = 0
        while attempts < 3:
            password = self._sudo_password
            if password is None:
                password = await self._prompt_sudo_password()
                if password is None:
                    return False

            if await self._validate_sudo_password(password):
                self._sudo_password = password
                return True

            self._sudo_password = None
            attempts += 1
            self.notify("Incorrect sudo password", severity="error")

        return False

    async def _send_sudo_password(self, process: asyncio.subprocess.Process) -> None:
        """Send the cached sudo password to a subprocess started with sudo -S."""
        if process.stdin is None or self._sudo_password is None:
            return
        process.stdin.write(f"{self._sudo_password}\n".encode())
        await process.stdin.drain()
        process.stdin.close()

    def _schedule_search(self) -> None:
        """Debounce background search requests while the user is typing."""
        if self._search_timer is not None:
            self._search_timer.stop()
        self._search_timer = self.set_timer(
            0.4,
            lambda: self.run_worker(
                self.search_new_packages(self.search_query), exclusive=False
            ),
        )

    def _background_refresh(self):
        """Refresh package list in background."""
        if not self._offline_mode:
            asyncio.ensure_future(self._silent_refresh())

    async def _silent_refresh(self):
        """Perform a silent background refresh without UI blocking."""
        try:
            await self.fetch_packages()
        except Exception:
            pass

    async def _background_fetch(self):
        """Background fetch for startup optimization."""
        try:
            await self.fetch_packages()
            if hasattr(self, "notify"):
                self.notify(
                    f"Package list refreshed - {len(self.all_packages)} packages",
                    severity="information",
                    timeout=2,
                )
        except Exception:
            pass
        finally:
            self._cache_refresh_in_progress = False

    def _update_package_table(self):
        """Update package table with current all_packages data."""
        try:
            table = self.query_one("#package-table", PackageTable)
            table.populate(self._get_filtered_packages(), self.favorites)
        except Exception:
            pass

    def _get_filtered_packages(self) -> List[Package]:
        """Get packages visible in the current source, mode, and search state.

        Returns:
            List of packages matching current filters
        """
        if not self.all_packages:
            return []

        filtered = self.all_packages

        if self.current_mode == "mode-updates":
            filtered = [p for p in filtered if p.status == PackageStatus.UPDATE]
        elif self.current_mode == "mode-search":
            filtered = [p for p in filtered if p.status == PackageStatus.NOT_INSTALLED]

        if self.current_source != "all":
            if self.current_source == "favorites":
                filtered = [p for p in filtered if p.row_key in self.favorites]
            else:
                filtered = [p for p in filtered if p.source == self.current_source]

        if self.search_query:
            q = self.search_query.lower()
            filtered = [
                p
                for p in filtered
                if q in p.name.lower() or q in (p.description or "").lower()
            ]

        return filtered

    def _get_table_cursor_row_key(self, table: PackageTable) -> Optional[str]:
        """Return the current table row key when the cursor is on a valid row."""
        if table.row_count == 0:
            return None
        if table.cursor_row < 0 or table.cursor_row >= table.row_count:
            return None
        return table.ordered_rows[table.cursor_row].key.value

    async def _check_network(self):
        """Check network connectivity without blocking the TUI."""
        try:
            import urllib.request

            await asyncio.to_thread(
                lambda: urllib.request.urlopen("https://pypi.org", timeout=3).close()
            )
            if self._auto_offline_mode:
                self._auto_offline_mode = False
                self._update_effective_offline_mode()
                self.notify("Back online", severity="information")
        except (urllib.error.URLError, OSError, TimeoutError):
            if not self._auto_offline_mode:
                self._auto_offline_mode = True
                self._update_effective_offline_mode()
                self.notify(
                    "Offline mode - remote operations disabled", severity="warning"
                )

    async def fetch_packages(self):
        """Asynchronously fetch packages without blocking event loop."""
        if self._fetch_in_progress:
            return

        self._fetch_in_progress = True
        packages: List[Package] = []
        try:
            if self._force_refresh:
                clear_cache()
                self._force_refresh = False

            def log_msg(msg: str):
                try:
                    self.query_one("#loading-msg", Label).update(msg)
                except Exception:
                    pass
                try:
                    self.query_one("#term-log", RichLog).write(
                        f"[cyan]INFO:[/] {msg}"
                    )
                except Exception:
                    pass

            async def run_cmd(cmd: List[str]) -> Tuple[int, str]:
                proc = await asyncio.create_subprocess_exec(
                    *cmd,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE,
                )
                stdout, _ = await asyncio.wait_for(proc.communicate(), timeout=60)
                return proc.returncode, stdout.decode(errors="ignore")

            if shutil.which("apt"):
                log_msg("Fetching APT packages...")
                try:
                    code, out = await run_cmd(["apt", "list", "--installed"])
                    if code == 0:
                        for line in out.splitlines():
                            if "/" in line and not line.startswith("Listing"):
                                parts = line.split()
                                name = parts[0].split("/")[0]
                                version = parts[1] if len(parts) > 1 else "?"
                                packages.append(
                                    Package(
                                        name,
                                        version,
                                        "apt",
                                        PackageStatus.INSTALLED,
                                        desc="Advanced Package Tool",
                                    )
                                )

                    code, out = await run_cmd(["apt", "list", "--upgradable"])
                    if code == 0:
                        for line in out.splitlines():
                            if "/" in line and not line.startswith("Listing"):
                                parts = line.split()
                                name = parts[0].split("/")[0]
                                ver = parts[1] if len(parts) > 1 else "?"
                                existing = next(
                                    (
                                        p
                                        for p in packages
                                        if p.name == name and p.source == "apt"
                                    ),
                                    None,
                                )
                                if existing:
                                    existing.status = PackageStatus.UPDATE
                                    existing.version = f"{existing.version} -> {ver}"
                                else:
                                    packages.append(
                                        Package(name, ver, "apt", PackageStatus.UPDATE)
                                    )
                except (OSError, asyncio.TimeoutError, ValueError) as e:
                    logger.log_exception(e, context="APT error")
                    log_msg(f"APT error: {e}")
            else:
                log_msg("Skipping APT: command not available")

            if shutil.which("flatpak"):
                log_msg("Fetching Flatpak packages...")
                try:
                    code, out = await run_cmd(["flatpak", "list", "--app"])
                    if code == 0:
                        for line in out.splitlines():
                            parts = line.split("\t")
                            if len(parts) >= 3:
                                packages.append(
                                    Package(
                                        parts[1],
                                        parts[2],
                                        "flatpak",
                                        PackageStatus.INSTALLED,
                                        desc=parts[0],
                                    )
                                )

                    code, out = await run_cmd(
                        ["flatpak", "remote-ls", "--updates", "--app"]
                    )
                    if code == 0:
                        for line in out.splitlines():
                            parts = line.split("\t")
                            if len(parts) < 2:
                                continue
                            name = parts[1].strip()
                            if not name:
                                continue
                            new_version = parts[2].strip() if len(parts) > 2 else ""
                            existing = next(
                                (
                                    pkg
                                    for pkg in packages
                                    if pkg.source == "flatpak" and pkg.name == name
                                ),
                                None,
                            )
                            if existing:
                                existing.status = PackageStatus.UPDATE
                                if new_version:
                                    existing.version = (
                                        f"{existing.version} -> {new_version}"
                                    )
                except (OSError, asyncio.TimeoutError, ValueError) as e:
                    logger.log_exception(e, context="Flatpak error")
                    log_msg(f"Flatpak error: {e}")
            else:
                log_msg("Skipping Flatpak: command not available")

            if shutil.which("cargo"):
                log_msg("Fetching Cargo packages...")
                try:
                    code, out = await run_cmd(["cargo", "install", "--list"])
                    if code == 0:
                        for line in out.splitlines():
                            match = re.match(r"(\S+)\s+v([\w.\-]+)", line)
                            if match:
                                packages.append(
                                    Package(
                                        match.group(1),
                                        match.group(2),
                                        "cargo",
                                        PackageStatus.INSTALLED,
                                        desc="Rust Crate",
                                    )
                                )
                except (OSError, asyncio.TimeoutError, ValueError) as e:
                    logger.log_exception(e, context="Cargo error")
                    log_msg(f"Cargo error: {e}")
            else:
                log_msg("Skipping Cargo: command not available")

            if shutil.which("npm"):
                log_msg("Fetching NPM packages...")
                try:
                    code, out = await run_cmd(["npm", "list", "-g", "--depth=0", "--json"])
                    if code == 0:
                        data = json.loads(out)
                        for name, info in data.get("dependencies", {}).items():
                            if name:
                                version = (
                                    info.get("version", "?")
                                    if isinstance(info, dict)
                                    else str(info)
                                )
                                packages.append(
                                    Package(
                                        name,
                                        version,
                                        "npm",
                                        PackageStatus.INSTALLED,
                                        desc="Node.js Package",
                                    )
                                )

                    outdated_code, outdated_out = await run_cmd(
                        ["npm", "outdated", "-g", "--depth=0", "--json"]
                    )
                    if outdated_code in (0, 1) and outdated_out.strip():
                        outdated_data = json.loads(outdated_out)
                        for name, info in outdated_data.items():
                            if not isinstance(info, dict):
                                continue
                            current = info.get("current") or "?"
                            wanted = info.get("wanted") or info.get("latest") or "?"
                            existing = next(
                                (
                                    pkg
                                    for pkg in packages
                                    if pkg.source == "npm" and pkg.name == name
                                ),
                                None,
                            )
                            if existing:
                                existing.status = PackageStatus.UPDATE
                                existing.version = f"{current} -> {wanted}"
                            else:
                                packages.append(
                                    Package(
                                        name,
                                        f"{current} -> {wanted}",
                                        "npm",
                                        PackageStatus.UPDATE,
                                        desc="Node.js Package",
                                    )
                                )
                except (
                    OSError,
                    asyncio.TimeoutError,
                    json.JSONDecodeError,
                    ValueError,
                ) as e:
                    logger.log_exception(e, context="NPM error")
                    log_msg(f"NPM error: {e}")
            else:
                log_msg("Skipping NPM: command not available")

            pip_cmd = self._get_system_pip_cmd()
            if pip_cmd:
                log_msg("Fetching PIP packages...")
                try:
                    code, out = await run_cmd(pip_cmd + ["list", "--format=json"])
                    if code == 0:
                        data = json.loads(out)
                        for pkg in data:
                            packages.append(
                                Package(
                                    pkg.get("name", "?"),
                                    pkg.get("version", "?"),
                                    "pip",
                                    PackageStatus.INSTALLED,
                                    desc="Python Package",
                                )
                            )

                    code, out = await run_cmd(
                        pip_cmd + ["list", "--outdated", "--format=json"]
                    )
                    if code == 0 and out.strip():
                        data = json.loads(out)
                        for pkg in data:
                            name = pkg.get("name", "?")
                            current = pkg.get("version", "?")
                            latest = pkg.get("latest_version") or pkg.get("latest") or "?"
                            existing = next(
                                (
                                    package
                                    for package in packages
                                    if package.source == "pip" and package.name == name
                                ),
                                None,
                            )
                            if existing:
                                existing.status = PackageStatus.UPDATE
                                existing.version = f"{current} -> {latest}"
                            else:
                                packages.append(
                                    Package(
                                        name,
                                        f"{current} -> {latest}",
                                        "pip",
                                        PackageStatus.UPDATE,
                                        desc="Python Package",
                                    )
                                )
                except (
                    OSError,
                    asyncio.TimeoutError,
                    json.JSONDecodeError,
                    ValueError,
                ) as e:
                    logger.log_exception(e, context="PIP error")
                    log_msg(f"PIP error: {e}")
            else:
                log_msg("Skipping PIP: system pip not available")

            if shutil.which("snap"):
                log_msg("Fetching Snap packages...")
                try:
                    code, out = await run_cmd(["snap", "list"])
                    if code == 0:
                        for line in out.splitlines()[1:]:
                            parts = line.split()
                            if len(parts) >= 1:
                                name = parts[0]
                                version = parts[1] if len(parts) > 1 else "?"
                                packages.append(
                                    Package(
                                        name,
                                        version,
                                        "snap",
                                        PackageStatus.INSTALLED,
                                        desc="Snap Package",
                                    )
                                )

                    code, out = await run_cmd(["snap", "refresh", "--list"])
                    if code == 0:
                        for line in out.splitlines()[1:]:
                            parts = line.split()
                            if len(parts) < 2:
                                continue
                            name = parts[0]
                            new_version = parts[1]
                            existing = next(
                                (
                                    pkg
                                    for pkg in packages
                                    if pkg.source == "snap" and pkg.name == name
                                ),
                                None,
                            )
                            if existing:
                                existing.status = PackageStatus.UPDATE
                                existing.version = f"{existing.version} -> {new_version}"
                except (OSError, asyncio.TimeoutError, ValueError) as e:
                    logger.log_exception(e, context="Snap error")
                    log_msg(f"Snap error: {e}")
            else:
                log_msg("Skipping Snap: command not available")

            aur_helper = self._resolve_aur_helper()
            if aur_helper:
                log_msg("Fetching AUR packages...")
                try:
                    code, out = await run_cmd([aur_helper, "-Q"])
                    if code == 0:
                        official_packages: Set[str] = set()
                        if shutil.which("pacman"):
                            code_official, out_official = await run_cmd(["pacman", "-Qn"])
                            if code_official == 0:
                                for line in out_official.splitlines():
                                    parts = line.split()
                                    if parts:
                                        official_packages.add(parts[0])

                        for line in out.splitlines():
                            parts = line.split()
                            if len(parts) >= 2:
                                name = parts[0]
                                version = parts[1]
                                if name not in official_packages:
                                    packages.append(
                                        Package(
                                            name,
                                            version,
                                            "aur",
                                            PackageStatus.INSTALLED,
                                            desc="AUR Package",
                                        )
                                    )
                except (OSError, asyncio.TimeoutError, ValueError) as e:
                    logger.log_exception(e, context="AUR error")
                    log_msg(f"AUR error: {e}")
            else:
                log_msg("Skipping AUR: no helper available")

            if shutil.which("dnf"):
                log_msg("Fetching DNF packages...")
                try:
                    code, out = await run_cmd(["dnf", "list", "installed"])
                    if code == 0:
                        for line in out.splitlines():
                            if line.startswith("Last metadata") or line.startswith(
                                "Installed"
                            ):
                                continue
                            parts = line.split()
                            if len(parts) >= 2 and "." in parts[0]:
                                name = parts[0].rsplit(".", 1)[0]
                                version = parts[1]
                                packages.append(
                                    Package(
                                        name,
                                        version,
                                        "dnf",
                                        PackageStatus.INSTALLED,
                                        desc="DNF Package",
                                    )
                                )

                    code, out = await run_cmd(["dnf", "check-update"])
                    if code in (0, 100):
                        for line in out.splitlines():
                            if (
                                not line
                                or line.startswith(" ")
                                or line.startswith("Last metadata")
                            ):
                                continue
                            parts = line.split()
                            if len(parts) >= 2 and "." in parts[0]:
                                name = parts[0].rsplit(".", 1)[0]
                                new_version = parts[1]
                                existing = next(
                                    (
                                        p
                                        for p in packages
                                        if p.name == name and p.source == "dnf"
                                    ),
                                    None,
                                )
                                if existing:
                                    existing.status = PackageStatus.UPDATE
                                    existing.version = (
                                        f"{existing.version} -> {new_version}"
                                    )
                except (OSError, asyncio.TimeoutError, ValueError) as e:
                    logger.log_exception(e, context="DNF error")
                    log_msg(f"DNF error: {e}")
            else:
                log_msg("Skipping DNF: command not available")

            if self._is_macos and self._has_brew:
                log_msg("Fetching Homebrew packages...")
                try:
                    code, out = await run_cmd(["brew", "list", "--versions", "--formula"])
                    if code == 0:
                        for line in out.splitlines():
                            parts = line.split()
                            if len(parts) >= 2:
                                packages.append(
                                    Package(
                                        parts[0],
                                        parts[1],
                                        "brew",
                                        PackageStatus.INSTALLED,
                                        desc="Homebrew Formula",
                                    )
                                )

                    code, out = await run_cmd(["brew", "list", "--versions", "--cask"])
                    if code == 0:
                        for line in out.splitlines():
                            parts = line.split()
                            if len(parts) >= 2:
                                packages.append(
                                    Package(
                                        parts[0],
                                        parts[1],
                                        "brew",
                                        PackageStatus.INSTALLED,
                                        desc="Homebrew Cask",
                                    )
                                )

                    code, out = await run_cmd(["brew", "outdated", "--quiet"])
                    if code == 0:
                        outdated_names = {
                            line.split()[0] for line in out.splitlines() if line.strip()
                        }
                        for pkg in packages:
                            if pkg.source == "brew" and pkg.name in outdated_names:
                                pkg.status = PackageStatus.UPDATE
                except (OSError, asyncio.TimeoutError, ValueError) as e:
                    logger.log_exception(e, context="Homebrew error")
                    log_msg(f"Homebrew error: {e}")

            if self._plugin_registry:
                log_msg("Fetching plugin packages...")
                try:
                    plugin_packages = await asyncio.to_thread(
                        self._plugin_registry.get_all_installed
                    )
                    if plugin_packages:
                        packages.extend(plugin_packages)
                        log_msg(f"Found {len(plugin_packages)} packages from plugins")
                except Exception as e:
                    logger.log_exception(e, context="Plugin error")

            self.all_packages = sorted(packages, key=lambda p: p.name.lower())
            save_cached_packages(self.all_packages)
            self.apply_filters()
            self._cache_refresh_in_progress = False
            if not self.all_packages:
                self.notify(
                    "No packages were discovered. Check the terminal log for backend errors.",
                    severity="warning",
                    timeout=6,
                )
            log_msg("Refresh complete.")
        finally:
            self._hide_loading_overlay()
            self._fetch_in_progress = False

    async def search_new_packages(self, query: str):
        """Search for new packages across repositories.

        Args:
            query: Search query string
        """
        request_id = self._search_request_id = self._search_request_id + 1
        is_valid, error_message = validation.validate_search_query(query)
        if not is_valid:
            self.notify(f"Invalid search query: {error_message}", severity="warning")
            return

        from linget.search import search_new_packages as do_search

        def log_msg(msg: str):
            try:
                self.query_one("#loading-msg", Label).update(msg)
            except Exception:
                pass
            try:
                self.query_one("#term-log", RichLog).write(
                    f"[cyan]SEARCH:[/] {msg}"
                )
            except Exception:
                pass

        log_msg(f"Searching for '{query}'...")
        found_packages = await do_search(query, self.all_packages, self.current_source)
        if request_id != self._search_request_id or query != self.search_query:
            return

        if self._plugin_registry and (
            self.current_source == "all"
            or self._plugin_registry.get(self.current_source)
        ):
            log_msg("Searching plugins...")
            try:
                plugin_results = await asyncio.to_thread(
                    self._plugin_registry.search_all, query
                )
                if plugin_results:
                    log_msg(f"Found {len(plugin_results)} packages from plugins")
                    found_packages.extend(plugin_results)
            except Exception as e:
                logger.log_exception(e, context="Plugin search error")

        if request_id != self._search_request_id or query != self.search_query:
            return

        searched_sources = {
            source
            for source in ("apt", "flatpak", "snap", "aur", "dnf", "brew")
            if self.current_source in ("all", source)
        }
        if self._plugin_registry:
            if self.current_source == "all":
                searched_sources.update(self._plugin_registry.plugin_names)
            elif self._plugin_registry.get(self.current_source):
                searched_sources.add(self.current_source)

        if searched_sources:
            self.all_packages = [
                p
                for p in self.all_packages
                if not (
                    p.status == PackageStatus.NOT_INSTALLED
                    and p.source in searched_sources
                )
            ]

        if found_packages:
            log_msg(f"Found {len(found_packages)} new packages")
            self.all_packages.extend(found_packages)
            self.all_packages = sorted(self.all_packages, key=lambda p: p.name.lower())
        else:
            log_msg("No new packages found")

        self.apply_filters()

    def apply_filters(self):
        """Apply current filters to package list and update table."""
        filtered = self._get_filtered_packages()

        table = self.query_one("#package-table", PackageTable)
        table.populate(filtered, self.favorites)

        if filtered and table.row_count > 0:
            table.move_cursor(row=0)
            self.update_info_panel(filtered[0])
        else:
            self._selected_package = None
            if self.search_query:
                self.notify(
                    f"No packages match '{self.search_query}'", severity="warning"
                )
            elif self.current_mode == "mode-updates":
                self.notify("No updates available", severity="information")
            elif self.current_mode == "mode-search":
                self.notify(
                    "Use 'Search for New' tab to find installable packages",
                    severity="information",
                )
            elif self.current_source == "favorites":
                self.notify("No favorite packages found", severity="information")

    def update_info_panel(self, package: Optional[Package]):
        """Track the selected package.

        Args:
            package: Package to display or None
        """
        self._selected_package = package

    def on_button_pressed(self, event: Button.Pressed) -> None:
        """Handle button press events.

        Args:
            event: Button pressed event
        """
        if event.button.id == "refresh-btn":
            self.action_refresh_data()

    def on_input_changed(self, event: Input.Changed) -> None:
        """Handle input change events.

        Args:
            event: Input changed event
        """
        if event.input.id == "search":
            self.search_query = event.value
            if self.current_mode == "mode-search" and len(self.search_query) >= 2:
                self._schedule_search()
            else:
                if self._search_timer is not None:
                    self._search_timer.stop()
                    self._search_timer = None
                self.apply_filters()

    def on_tabs_tab_activated(self, event: Tabs.TabActivated) -> None:
        """Handle tab activation events for mode and source tabs.

        Args:
            event: Tab activated event
        """
        tab_id = event.tab.id
        if tab_id in ("mode-all", "mode-updates", "mode-search"):
            self.current_mode = tab_id
            self._settings["default_mode"] = tab_id
            save_settings(self._settings)
            self.apply_filters()
        elif tab_id and tab_id.startswith("src-"):
            source = tab_id[4:]  # strip "src-" prefix
            plugin_names = (
                set(self._plugin_registry.plugin_names)
                if self._plugin_registry
                else set()
            )
            valid_sources = {
                "all", "apt", "flatpak", "snap", "cargo", "npm",
                "pip", "favorites", "aur", "dnf", "brew",
            } | plugin_names
            if source in valid_sources:
                self.current_source = source
                self._settings["default_source"] = source
                save_settings(self._settings)
                self.apply_filters()

    def on_data_table_row_highlighted(self, event: DataTable.RowHighlighted) -> None:
        """Handle row highlight events.

        Args:
            event: Row highlighted event
        """
        row_key = event.row_key.value
        pkg = next(
            (p for p in self.all_packages if f"{p.source}-{p.name}" == row_key), None
        )
        if pkg:
            self.update_info_panel(pkg)

    def action_focus_search(self):
        """Focus the search input."""
        search = self.query_one("#search", Input)
        search.focus()
        search.cursor_position = len(search.value)

    def action_command_palette(self):
        """Show the command palette."""
        self.push_screen(LingetCommandPalette())

    def action_refresh_data(self):
        """Refresh package data."""
        if self._offline_mode:
            self.notify("Offline mode - refresh disabled", severity="warning")
            return
        if self._fetch_in_progress:
            self.notify("Refresh already in progress", severity="warning")
            return
        self._show_loading_overlay()
        self.run_worker(self.fetch_packages(), exclusive=False)

    def action_force_refresh(self):
        """Force refresh package data ignoring cache."""
        if self._offline_mode:
            self.notify("Offline mode - refresh disabled", severity="warning")
            return
        if self._fetch_in_progress:
            self.notify("Refresh already in progress", severity="warning")
            return
        self._force_refresh = True
        self._show_loading_overlay()
        self.run_worker(self.fetch_packages(), exclusive=False)

    def _queue_task(self, action: str):
        """Queue a task for the selected package.

        Args:
            action: Action to perform (install, update, remove)
        """
        pkg = self._selected_package
        if not pkg:
            self.notify("No package selected!", severity="warning")
            return

        if action == "install":
            self._last_action = (pkg, "remove")
        elif action == "remove":
            self._last_action = (pkg, "install")
        elif action == "update":
            self._last_action = (pkg, "update")

        task = Task(pkg, action)
        self.tasks.append(task)

        queue_panel = self.query_one("#queue-panel", QueuePanel)
        queue_panel.add_task(task)
        self._sync_bottom_panel_state()

        try:
            term = self.query_one("#term-log", RichLog)
            term.write(f"[yellow]QUEUED:[/] {action.upper()} {pkg.name}")
        except Exception:
            pass
        self.notify(f"Queued: {action} {pkg.name}", severity="information")

        asyncio.ensure_future(self.run_task(task))

    def action_show_details(self):
        """Show package detail modal for selected package."""
        pkg = self._selected_package
        if not pkg:
            self.notify("No package selected", severity="warning")
            return

        def handle_result(action: str | None) -> None:
            if action == "install":
                self._queue_task("install")
            elif action == "update":
                self._queue_task("update")
            elif action == "remove":
                self.action_remove()
            elif action == "favorite":
                self.action_toggle_favorite()

        self.push_screen(
            PackageDetailScreen(pkg, self.favorites),
            handle_result,
        )

    def action_install(self):
        """Queue install action for selected package."""
        self._queue_task("install")

    def action_update(self):
        """Queue update action for selected package."""
        self._queue_task("update")

    def action_remove(self):
        """Queue remove action for selected package with confirmation."""
        pkg = self._selected_package
        if not pkg:
            self.notify("No package selected!", severity="warning")
            return

        now = time.monotonic()
        if (
            self._pending_remove is not None
            and self._pending_remove[0] == pkg.row_key
            and now - self._pending_remove[1] <= 4.0
        ):
            self._pending_remove = None
            self._queue_task("remove")
        else:
            self._pending_remove = (pkg.row_key, now)
            self.notify(
                f"Press 'r' again to confirm removing {pkg.name}",
                severity="error",
                timeout=3.0,
            )

    def action_clear_tasks(self):
        """Clear completed tasks from queue."""
        completed_statuses = {"done", "error", "cancelled"}
        to_remove = [t for t in self.tasks if t.status in completed_statuses]
        for t in to_remove:
            try:
                row = self.query_one(f"#task-row-{t.dom_id}")
                row.remove()
            except Exception:
                pass

        self.tasks = [t for t in self.tasks if t.status not in completed_statuses]

        queue_panel = self.query_one("#queue-panel", QueuePanel)
        existing = queue_panel.query("#empty-queue")
        if not existing and not self.tasks:
            queue_panel.mount(
                Label("No active tasks.", id="empty-queue", classes="dim")
            )

        self._sync_bottom_panel_state()
        self.notify("Cleared completed tasks")

    def action_cancel_task(self):
        """Cancel the currently running task."""
        running_task = None
        for task in self.tasks:
            if task.status == "running":
                running_task = task
                break

        if not running_task:
            self.notify("No running task to cancel", severity="warning")
            return

        process = self._running_tasks.get(running_task.id)
        if process:
            try:
                process.terminate()
                running_task.status = "cancelled"
                running_task.error_type = ErrorType.USER_CANCELLED
                running_task.error_message = "Operation cancelled by user"
                try:
                    row = self.query_one(
                        f"#task-row-{running_task.dom_id}", TaskRow
                    )
                    row.update_progress(running_task.progress, running_task.status)
                except Exception:
                    pass
                self._sync_bottom_panel_state()
                self.notify(f"Cancelled: {running_task.package.name}")
                try:
                    self.query_one("#term-log", RichLog).write(
                        f"[yellow]CANCELLED:[/] {running_task.action.upper()} {running_task.package.name}"
                    )
                except Exception:
                    pass
            except (OSError, ProcessLookupError) as e:
                self.notify(f"Failed to cancel: {e}", severity="error")

    def action_retry_task(self):
        """Retry the last failed task."""
        failed_tasks = [t for t in self.tasks if t.status == "error"]
        if not failed_tasks:
            self.notify("No failed tasks to retry", severity="warning")
            return

        task_to_retry = failed_tasks[-1]
        new_task = Task(task_to_retry.package, task_to_retry.action)
        self.tasks.append(new_task)

        queue_panel = self.query_one("#queue-panel", QueuePanel)
        queue_panel.add_task(new_task)
        self._sync_bottom_panel_state()

        try:
            self.query_one("#term-log", RichLog).write(
                f"[yellow]RETRY:[/] {new_task.action.upper()} {new_task.package.name}"
            )
        except Exception:
            pass
        self.notify(
            f"Retrying: {new_task.action} {new_task.package.name}",
            severity="information",
        )

        asyncio.ensure_future(self.run_task(new_task))

    async def action_show_dependencies(self):
        """Show package dependencies."""
        pkg = self._selected_package
        if not pkg:
            self.notify("No package selected!", severity="warning")
            return

        try:
            self.query_one("#term-log", RichLog).write(
                f"[cyan]DEPS:[/] Fetching dependencies for {pkg.name}..."
            )
        except Exception:
            pass

        deps: List[str] = []
        reverse_deps: List[str] = []

        if pkg.source == "apt":
            try:
                proc = await asyncio.create_subprocess_exec(
                    "apt-cache",
                    "depends",
                    pkg.name,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE,
                )
                stdout, _ = await proc.communicate()
                if proc.returncode == 0:
                    for line in stdout.decode().splitlines():
                        if line.startswith("  Depends:"):
                            dep = line.replace("  Depends:", "").strip()
                            deps.append(dep)
                        elif line.startswith("  Recommends:"):
                            dep = line.replace("  Recommends:", "").strip()
                            deps.append(f"{dep} (recommended)")
            except (OSError, asyncio.TimeoutError, ValueError) as e:
                logger.log_exception(e, context="Error fetching deps")
                self.notify(f"Error fetching deps: {e}", severity="error")

            try:
                proc = await asyncio.create_subprocess_exec(
                    "apt-cache",
                    "rdepends",
                    pkg.name,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE,
                )
                stdout, _ = await proc.communicate()
                if proc.returncode == 0:
                    for line in stdout.decode().splitlines():
                        if line.startswith("  ") and not line.startswith("   "):
                            reverse_deps.append(line.strip())
            except (OSError, asyncio.TimeoutError):
                pass

        try:
            log = self.query_one("#term-log", RichLog)
            log.write(f"[bold]Dependencies for {pkg.name}:[/]")
            if deps:
                for dep in deps[:10]:
                    log.write(f"  • {dep}")
                if len(deps) > 10:
                    log.write(f"  ... and {len(deps) - 10} more")
            else:
                log.write("  No dependencies found")

            if reverse_deps:
                log.write(f"\n[bold]Required by:[/]")
                for rdep in reverse_deps[:5]:
                    log.write(f"  • {rdep}")
                if len(reverse_deps) > 5:
                    log.write(f"  ... and {len(reverse_deps) - 5} more")
        except Exception:
            pass

        self.notify(f"Dependencies shown for {pkg.name}")

    def action_toggle_select(self):
        """Toggle selection of current package."""
        table = self.query_one("#package-table", PackageTable)
        row_key = self._get_table_cursor_row_key(table)
        if row_key is None:
            return

        if row_key in table.selected_rows:
            table.selected_rows.remove(row_key)
            self.selected_packages.discard(row_key)
        else:
            table.selected_rows.add(row_key)
            self.selected_packages.add(row_key)

        self.apply_filters()
        self.notify(f"Selected: {len(self.selected_packages)} packages")

    def action_select_all(self):
        """Select all visible packages."""
        table = self.query_one("#package-table", PackageTable)
        for pkg in self._get_filtered_packages():
            row_key = f"{pkg.source}-{pkg.name}"
            table.selected_rows.add(row_key)
            self.selected_packages.add(row_key)

        self.apply_filters()
        self.notify(f"Selected all: {len(self.selected_packages)} packages")

    def action_deselect_all(self):
        """Clear all selections."""
        table = self.query_one("#package-table", PackageTable)
        table.selected_rows.clear()
        self.selected_packages.clear()
        self.apply_filters()
        self.notify("Cleared all selections")

    def _confirm_bulk_operation(self, packages: List[Package], action: str) -> bool:
        """Confirm bulk operation if more than 5 packages.

        Args:
            packages: List of packages to operate on
            action: Action being performed

        Returns:
            True if confirmed or no confirmation needed
        """
        if len(packages) > 5:
            package_keys = frozenset(pkg.row_key for pkg in packages)
            self.notify(
                f"Bulk {action}: {len(packages)} packages. Press {action.upper()} again to confirm.",
                severity="warning",
                timeout=5.0,
            )
            attr_name = f"_pending_bulk_{action}"
            if hasattr(self, attr_name) and getattr(self, attr_name) == package_keys:
                setattr(self, attr_name, None)
                return True
            setattr(self, attr_name, package_keys)
            return False
        return True

    def action_bulk_install(self):
        """Bulk install selected packages."""
        if not self.selected_packages:
            self.notify(
                "No packages selected! Press SPACE to select.", severity="warning"
            )
            return

        packages_to_install: List[Package] = []
        for row_key in self.selected_packages:
            source, name = row_key.split("-", 1)
            pkg = next(
                (p for p in self.all_packages if p.source == source and p.name == name),
                None,
            )
            if pkg and pkg.status == PackageStatus.NOT_INSTALLED:
                packages_to_install.append(pkg)

        if not packages_to_install:
            self.notify("No installable packages selected", severity="warning")
            return

        if not self._confirm_bulk_operation(packages_to_install, "install"):
            return

        for pkg in packages_to_install:
            task = Task(pkg, "install")
            self.tasks.append(task)
            self.query_one("#queue-panel", QueuePanel).add_task(task)
            self._sync_bottom_panel_state()
            asyncio.ensure_future(self.run_task(task))

        self.notify(f"Bulk installing {len(packages_to_install)} packages...")

    def action_bulk_update(self):
        """Bulk update selected packages."""
        if not self.selected_packages:
            self.notify(
                "No packages selected! Press SPACE to select.", severity="warning"
            )
            return

        packages_to_update: List[Package] = []
        for row_key in self.selected_packages:
            source, name = row_key.split("-", 1)
            pkg = next(
                (p for p in self.all_packages if p.source == source and p.name == name),
                None,
            )
            if pkg and pkg.status == PackageStatus.UPDATE:
                packages_to_update.append(pkg)

        if not packages_to_update:
            self.notify("No updatable packages selected", severity="warning")
            return

        if not self._confirm_bulk_operation(packages_to_update, "update"):
            return

        for pkg in packages_to_update:
            task = Task(pkg, "update")
            self.tasks.append(task)
            self.query_one("#queue-panel", QueuePanel).add_task(task)
            self._sync_bottom_panel_state()
            asyncio.ensure_future(self.run_task(task))

        self.notify(f"Bulk updating {len(packages_to_update)} packages...")

    def action_undo(self):
        """Undo the last action by reversing it."""
        if not self._last_action:
            self.notify("Nothing to undo", severity="warning")
            return

        pkg, reverse_action = self._last_action

        if reverse_action == "update":
            self.notify("Cannot undo updates automatically", severity="error")
            return

        task = Task(pkg, reverse_action)
        self.tasks.append(task)

        queue_panel = self.query_one("#queue-panel", QueuePanel)
        queue_panel.add_task(task)
        self._sync_bottom_panel_state()

        try:
            term = self.query_one("#term-log", RichLog)
            term.write(f"[yellow]UNDO:[/] {reverse_action.upper()} {pkg.name}")
        except Exception:
            pass
        self.notify(f"Undoing: {reverse_action} {pkg.name}", severity="information")

        asyncio.ensure_future(self.run_task(task))
        self._last_action = None

    def action_toggle_favorite(self):
        """Toggle favorite status for the currently selected package."""
        pkg = self._selected_package
        if not pkg:
            self.notify("No package selected!", severity="warning")
            return

        row_key = pkg.row_key
        if row_key in self.favorites:
            self.favorites.remove(row_key)
            self.notify(f"Removed {pkg.name} from favorites", severity="information")
        else:
            self.favorites.add(row_key)
            self.notify(f"Added {pkg.name} to favorites", severity="information")

        save_favorites(self.favorites)
        self.apply_filters()

    async def action_show_orphans(self):
        """Show orphan packages that can be auto-removed."""
        if self.current_source not in ("all", "apt"):
            self.notify(
                f"Orphan detection is only supported for APT packages, not {self.current_source}",
                severity="warning",
            )
            return
        if not shutil.which("apt-get"):
            self.notify("APT is not available on this system", severity="warning")
            return

        self.notify("Checking for orphan packages...", severity="information")

        try:
            proc = await asyncio.create_subprocess_exec(
                "apt-get",
                "autoremove",
                "--dry-run",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, _ = await proc.communicate()

            if proc.returncode == 0:
                output = stdout.decode()
                orphans: List[str] = []

                for line in output.splitlines():
                    if "Remv " in line or "Remove " in line:
                        parts = line.split()
                        for i, part in enumerate(parts):
                            if part in ("Remv", "Remove") and i + 1 < len(parts):
                                pkg_name = parts[i + 1].split("-")[0].split("=")[0]
                                if pkg_name and pkg_name not in orphans:
                                    orphans.append(pkg_name)

                try:
                    log = self.query_one("#term-log", RichLog)
                    if orphans:
                        log.write(
                            f"[bold yellow]Orphan packages ({len(orphans)}):[/]"
                        )
                        for orphan in orphans[:20]:
                            log.write(f"  • {orphan}")
                        if len(orphans) > 20:
                            log.write(f"  ... and {len(orphans) - 20} more")

                        self.notify(
                            f"Found {len(orphans)} orphan packages. Run 'sudo apt autoremove' to clean up.",
                            severity="warning",
                            timeout=10.0,
                        )
                    else:
                        log.write("[green]No orphan packages found[/]")
                        self.notify("No orphan packages found", severity="information")
                except Exception:
                    pass
            else:
                self.notify("Failed to check for orphans", severity="error")

        except (OSError, asyncio.TimeoutError, ValueError) as e:
            logger.log_exception(e, context="Error checking orphans")
            self.notify(f"Error checking orphans: {e}", severity="error")

    async def action_clean_cache(self):
        """Clean package manager cache for the current source."""
        if self.current_source == "all":
            self.notify("Select a specific source to clean cache", severity="warning")
            return

        pip_cmd = self._get_system_pip_cmd()
        aur_helper = self._resolve_aur_helper()
        cache_configs = {
            "apt": [
                self._build_privileged_cmd("apt-get", "clean"),
                self._build_privileged_cmd("apt-get", "autoclean"),
            ],
            "flatpak": [["flatpak", "uninstall", "--unused", "-y"]],
            "cargo": [["cargo", "cache", "--autoclean"]],
            "npm": [["npm", "cache", "clean", "--force"]],
            "pip": [pip_cmd + ["cache", "purge"]] if pip_cmd else [],
            "aur": [[aur_helper, "-Sc", "--noconfirm"]] if aur_helper else [],
            "dnf": [self._build_privileged_cmd("dnf", "clean", "all")],
        }

        if self.current_source not in cache_configs or not cache_configs[self.current_source]:
            self.notify(
                f"Cache cleaning not supported for {self.current_source}",
                severity="warning",
            )
            return

        now = time.monotonic()
        if (
            self._pending_clean_cache is not None
            and self._pending_clean_cache[0] == self.current_source
            and now - self._pending_clean_cache[1] <= 4.0
        ):
            self._pending_clean_cache = None
        else:
            self.notify(
                f"Press 'X' again to confirm cleaning {self.current_source} cache",
                severity="warning",
                timeout=3.0,
            )
            self._pending_clean_cache = (self.current_source, now)
            return

        log: Optional[RichLog] = None
        try:
            log = self.query_one("#term-log", RichLog)
            log.write(f"[cyan]Cleaning {self.current_source} cache...[/]")
        except Exception:
            pass

        if any(cmd and cmd[0] == "sudo" for cmd in cache_configs[self.current_source]):
            if not await self._ensure_sudo_password():
                self.notify("Cache cleaning cancelled", severity="warning")
                return

        before_size = None
        if self.current_source == "apt":
            try:
                proc = await asyncio.create_subprocess_exec(
                    "du",
                    "-sh",
                    "/var/cache/apt/archives",
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE,
                )
                stdout, _ = await proc.communicate()
                if proc.returncode == 0:
                    before_size = stdout.decode().strip().split()[0]
                    if log is not None:
                        log.write(f"[dim]Cache size before: {before_size}[/]")
            except (OSError, ValueError):
                pass

        commands = cache_configs[self.current_source]
        all_succeeded = True
        for cmd in commands:
            try:
                proc = await asyncio.create_subprocess_exec(
                    *cmd,
                    stdin=asyncio.subprocess.PIPE if cmd and cmd[0] == "sudo" else None,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.STDOUT,
                )
                if cmd and cmd[0] == "sudo":
                    await self._send_sudo_password(proc)
                stdout, _ = await proc.communicate()
                if proc.returncode == 0:
                    output = stdout.decode().strip()
                    if output and log is not None:
                        for line in output.splitlines()[:20]:
                            log.write(f"  {line}")
                else:
                    output = stdout.decode("utf-8", errors="replace").strip()
                    if cmd and cmd[0] == "sudo" and (
                        "sorry, try again" in output.lower()
                        or "incorrect password" in output.lower()
                        or "no password was provided" in output.lower()
                    ):
                        self._sudo_password = None
                    all_succeeded = False
                    if log is not None:
                        log.write(f"[red]Command failed: {' '.join(cmd)}[/]")
            except (OSError, asyncio.TimeoutError, ValueError) as e:
                all_succeeded = False
                if log is not None:
                    log.write(f"[red]Error running {' '.join(cmd)}: {e}[/]")

        if self.current_source == "apt" and before_size:
            try:
                proc = await asyncio.create_subprocess_exec(
                    "du",
                    "-sh",
                    "/var/cache/apt/archives",
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE,
                )
                stdout, _ = await proc.communicate()
                if proc.returncode == 0:
                    after_size = stdout.decode().strip().split()[0]
                    if log is not None:
                        log.write(f"[dim]Cache size after: {after_size}[/]")
                    self.notify(
                        f"APT cache cleaned (was: {before_size}, now: {after_size})",
                        severity="information",
                        timeout=5.0,
                    )
                    return
            except (OSError, ValueError):
                pass

        if all_succeeded:
            self.notify(f"{self.current_source} cache cleaned", severity="information")
        else:
            self.notify(
                f"{self.current_source} cache cleaning finished with errors",
                severity="warning",
            )

    async def action_show_versions(self):
        """Show package version history using apt-cache policy."""
        try:
            log = self.query_one("#term-log", RichLog)
        except Exception:
            return

        try:
            package = self._selected_package

            if not package:
                self.notify("No package selected", severity="warning")
                return

            if package.source != "apt":
                self.notify(
                    "Version history only available for apt packages",
                    severity="warning",
                )
                return

            log.write(f"[cyan]Fetching version history for {package.name}...[/]")

            process = await asyncio.create_subprocess_exec(
                "apt-cache",
                "policy",
                package.name,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await process.communicate()

            if process.returncode == 0:
                output = stdout.decode("utf-8", errors="replace")
                lines = output.strip().split("\n")

                log.write(f"[bold green]Version history for {package.name}:[/]")

                installed = None
                candidate = None
                available: List[Tuple[str, str, bool]] = []

                for line in lines:
                    line = line.strip()
                    if line.startswith("Installed:"):
                        installed = line.split(":", 1)[1].strip()
                    elif line.startswith("Candidate:"):
                        candidate = line.split(":", 1)[1].strip()
                    elif line.startswith("***"):
                        parts = line.split()
                        if len(parts) >= 2:
                            version = parts[1]
                            repo = " ".join(parts[3:]) if len(parts) > 3 else ""
                            available.append((version, repo, True))
                    elif line.startswith(" ") and not line.startswith("   "):
                        parts = line.split()
                        if len(parts) >= 2:
                            version = parts[0]
                            repo = " ".join(parts[2:]) if len(parts) > 2 else ""
                            available.append((version, repo, False))

                if installed and installed != "(none)":
                    log.write(f"  Installed: {installed}")
                else:
                    log.write("  Installed: (none)")

                if candidate and candidate != "(none)":
                    log.write(f"  Candidate: {candidate}")
                else:
                    log.write("  Candidate: (none)")

                if available:
                    log.write("  Available:")
                    for version, repo, is_installed in available:
                        if is_installed:
                            log.write(f"    • {version} (installed)")
                        else:
                            display_repo = repo.strip("()") if repo else "unknown"
                            log.write(f"    • {version} ({display_repo})")
                else:
                    log.write("  No versions available in repositories")

                self.notify(
                    f"Version history for {package.name} displayed",
                    severity="information",
                )
            else:
                error_msg = stderr.decode("utf-8", errors="replace").strip()
                log.write(f"[red]Failed to fetch versions: {error_msg}[/]")
                self.notify(
                    f"Version history unavailable for {package.name}",
                    severity="warning",
                )

        except Exception as e:
            logger.log_exception(e, context="Error fetching versions")
            log.write(f"[red]Error fetching versions: {e}[/]")
            self.notify(f"Error fetching versions: {e}", severity="error")

    async def run_task(self, task: Task):
        """Execute real package manager commands natively in the async event loop.

        Args:
            task: Task object containing package and action information
        """
        self._running_tasks[task.id] = None

        def log_msg(msg: str):
            try:
                self.query_one("#term-log", RichLog).write(
                    f"[{task.package.source}] {msg}"
                )
            except Exception:
                pass

        def update_status(status: str, progress: Optional[float] = None):
            task.status = status
            if progress is not None:
                task.progress = min(progress, 100.0)
            try:
                row = self.query_one(f"#task-row-{task.dom_id}", TaskRow)
                row.update_progress(task.progress, task.status)
            except Exception:
                pass

        update_status("running", 5.0)
        try:
            self.query_one("#term-log", RichLog).write(
                f"[green]STARTED:[/] {task.action.upper()} {task.package.name}"
            )
        except Exception:
            pass

        cmd: List[str] = []
        source = task.package.source
        name = task.package.name
        action = task.action
        apt_base = self._build_privileged_cmd("apt-get", "-y")
        snap_base = self._build_privileged_cmd("snap")
        dnf_base = self._build_privileged_cmd("dnf", "-y")

        if source == "apt":
            if action == "install":
                cmd = apt_base + ["install", name]
            elif action == "remove":
                cmd = apt_base + ["remove", name]
            elif action == "update":
                cmd = apt_base + ["install", "--only-upgrade", name]
        elif source == "flatpak":
            if action == "install":
                cmd = ["flatpak", "install", "-y", name]
            elif action == "remove":
                cmd = ["flatpak", "uninstall", "-y", name]
            elif action == "update":
                cmd = ["flatpak", "update", "-y", name]
        elif source == "cargo":
            if action == "install":
                cmd = ["cargo", "install", name]
            elif action == "update":
                cmd = ["cargo", "install", "--force", name]
            elif action == "remove":
                cmd = ["cargo", "uninstall", name]
        elif source == "npm":
            if action in ("install", "update"):
                cmd = ["npm", "install", "-g", name]
            elif action == "remove":
                cmd = ["npm", "uninstall", "-g", name]
        elif source == "pip":
            pip_cmd = self._get_system_pip_cmd()
            if not pip_cmd:
                cmd = []
            else:
                extra = ["--break-system-packages"] if self._pip_is_externally_managed() else []
                if action in ("install", "update"):
                    cmd = pip_cmd + ["install", "--upgrade"] + extra + [name]
                elif action == "remove":
                    cmd = pip_cmd + ["uninstall", "-y"] + extra + [name]
        elif source == "snap":
            if action == "install":
                cmd = snap_base + ["install", name]
            elif action == "remove":
                cmd = snap_base + ["remove", name]
            elif action == "update":
                cmd = snap_base + ["refresh", name]
        elif source == "aur":
            aur_helper = self._resolve_aur_helper()
            if aur_helper:
                if action in ("install", "update"):
                    cmd = [aur_helper, "-S", "--noconfirm", name]
                elif action == "remove":
                    cmd = [aur_helper, "-R", "--noconfirm", name]
        elif source == "dnf":
            if action == "install":
                cmd = dnf_base + ["install", name]
            elif action == "remove":
                cmd = dnf_base + ["remove", name]
            elif action == "update":
                cmd = dnf_base + ["upgrade", name]
        elif source == "brew":
            if action == "install":
                cmd = ["brew", "install", name]
            elif action == "remove":
                cmd = ["brew", "uninstall", name]
            elif action == "update":
                cmd = ["brew", "upgrade", name]

        plugin = self._plugin_registry.get(source) if self._plugin_registry else None
        if plugin and not cmd:
            try:
                update_status("running", 20.0)
                success = await asyncio.to_thread(
                    {
                        "install": plugin.install,
                        "remove": plugin.remove,
                        "update": plugin.update,
                    }[action],
                    task.package,
                )
                if success:
                    update_status("done", 100.0)
                    self.notify(f"Completed: {action} {name}")
                    self.action_refresh_data()
                else:
                    task.error_type = ErrorType.UNKNOWN
                    task.error_message = "Plugin backend reported failure"
                    update_status("error")
                    self.notify(
                        f"Failed ({task.error_type.value}): {action} {name}",
                        severity="error",
                    )
                return
            except Exception as e:
                task.error_type = ErrorType.UNKNOWN
                task.error_message = str(e)
                update_status("error")
                logger.log_exception(e, context=f"Plugin task error for {source}")
                self.notify(
                    f"Failed ({task.error_type.value}): {action} {name}",
                    severity="error",
                )
                return

        if not cmd:
            log_msg(f"[red]Error:[/] Unsupported action/source combination.")
            update_status("error")
            return

        requires_privilege = bool(cmd) and cmd[0] in {"pkexec", "sudo"}
        has_graphical_auth = bool(
            os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY")
        )
        uses_sudo = bool(cmd) and cmd[0] == "sudo"
        auth_failed = False

        try:
            if bool(cmd) and cmd[0] == "pkexec" and not has_graphical_auth:
                task.error_type = ErrorType.PERMISSION
                task.error_message = (
                    "Graphical authentication is unavailable in this terminal session. "
                    "Run LinGet inside a desktop session or use the system package manager directly."
                )
                update_status("error")
                log_msg(f"[red]Auth unavailable:[/] {task.error_message}")
                self.notify(
                    f"Authentication unavailable for {action} {name}",
                    severity="error",
                    timeout=6,
                )
                return

            if uses_sudo and not await self._ensure_sudo_password():
                task.error_type = ErrorType.AUTH_CANCELLED
                task.error_message = "Sudo authentication was cancelled"
                update_status("error")
                log_msg(f"[red]Authentication cancelled:[/] {task.error_message}")
                self.notify(
                    f"Authentication cancelled for {action} {name}",
                    severity="warning",
                    timeout=6,
                )
                return

            if source == "apt" and requires_privilege:
                update_status("running", 10.0)
                log_msg("[dim]Waiting for authentication...[/]")

            process = await asyncio.wait_for(
                asyncio.create_subprocess_exec(
                    *cmd,
                    stdin=asyncio.subprocess.PIPE if uses_sudo else None,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.STDOUT,
                ),
                timeout=DEFAULT_TIMEOUT,
            )

            if uses_sudo:
                await self._send_sudo_password(process)

            self._running_tasks[task.id] = process
            saw_output = False
            deadline = asyncio.get_running_loop().time() + DEFAULT_TIMEOUT

            while True:
                remaining = deadline - asyncio.get_running_loop().time()
                if remaining <= 0:
                    raise asyncio.TimeoutError

                try:
                    line_timeout = remaining
                    if bool(cmd) and cmd[0] == "pkexec" and not saw_output:
                        line_timeout = min(line_timeout, 20)
                    line = await asyncio.wait_for(
                        process.stdout.readline(),
                        timeout=line_timeout,
                    )
                except asyncio.TimeoutError:
                    try:
                        process.kill()
                        await process.wait()
                    except (OSError, ProcessLookupError):
                        pass
                    if bool(cmd) and cmd[0] == "pkexec" and not saw_output:
                        task.error_type = ErrorType.AUTH_CANCELLED
                        task.error_message = (
                            "Authentication prompt was not completed. "
                            "If no polkit dialog appeared, launch LinGet from a graphical desktop session."
                        )
                        update_status("error")
                        log_msg(
                            f"[red]Authentication timeout:[/] {task.error_message}"
                        )
                        self.notify(
                            f"Authentication timed out for {action} {name}",
                            severity="error",
                            timeout=6,
                        )
                        return

                    task.error_type = ErrorType.TIMEOUT
                    task.error_message = (
                        f"Operation timed out after {DEFAULT_TIMEOUT} seconds"
                    )
                    update_status("error")
                    log_msg(f"[red]Timeout:[/] {task.error_message}")
                    self.notify(f"Timeout: {action} {name}", severity="error")
                    return
                if not line:
                    break

                text_line = line.decode("utf-8", errors="replace").strip()
                if text_line:
                    saw_output = True
                    lowered = text_line.lower()
                    if uses_sudo and (
                        "sorry, try again" in lowered
                        or "incorrect password" in lowered
                        or "no password was provided" in lowered
                    ):
                        auth_failed = True
                    log_msg(text_line)
                    if task.progress < 95:
                        update_status("running", min(task.progress + 1.5, 95.0))

            await process.wait()
            if task.status == "cancelled":
                return
            return_code = process.returncode

            if return_code == 0:
                update_status("done", 100.0)
                self.notify(f"Completed: {action} {name}")
                try:
                    self.query_one("#term-log", RichLog).write(
                        f"[bold green]COMPLETED:[/] {task.action.upper()} {task.package.name}"
                    )
                except Exception:
                    pass
                self.action_refresh_data()
            else:
                if uses_sudo and auth_failed:
                    self._sudo_password = None
                    task.error_type = ErrorType.PERMISSION
                    task.error_message = "Incorrect sudo password"
                elif return_code == 126 or return_code == 127:
                    task.error_type = ErrorType.NOT_FOUND
                    task.error_message = (
                        f"Command not found or not executable (exit {return_code})"
                    )
                elif return_code == 1 and source == "apt":
                    task.error_type = ErrorType.CONFLICT
                    task.error_message = "Package conflict or dependency issue"
                elif return_code == 100:
                    task.error_type = ErrorType.LOCKED
                    task.error_message = "dpkg/apt is locked by another process"
                else:
                    task.error_type = ErrorType.UNKNOWN
                    task.error_message = f"Failed with exit code {return_code}"

                update_status("error")
                log_msg(
                    f"[red]Failed [{task.error_type.value}]:[/] {task.error_message}"
                )
                self.notify(
                    f"Failed ({task.error_type.value}): {action} {name}",
                    severity="error",
                )

        except asyncio.TimeoutError:
            task.error_type = ErrorType.TIMEOUT
            task.error_message = f"Operation timed out after {DEFAULT_TIMEOUT} seconds"
            update_status("error")
            log_msg(f"[red]Timeout:[/] {task.error_message}")
            self.notify(f"Timeout: {action} {name}", severity="error")
            if task.id in self._running_tasks:
                try:
                    self._running_tasks[task.id].kill()
                except (OSError, ProcessLookupError):
                    pass

        except (OSError, ValueError, RuntimeError) as e:
            error_str = str(e).lower()
            error_msg = str(e)

            if "cancel" in error_str or "terminate" in error_str:
                task.error_type = ErrorType.AUTH_CANCELLED
                task.error_message = "Operation cancelled by user"
            elif "lock" in error_str or ("dpkg" in error_str and "lock" in error_str):
                task.error_type = ErrorType.LOCKED
                task.error_message = "Package manager is locked by another process"
            elif (
                "network" in error_str
                or "timeout" in error_str
                or "connection" in error_str
            ):
                task.error_type = ErrorType.NETWORK
                task.error_message = "Network error or timeout"
            elif "not found" in error_str or "no package" in error_str:
                task.error_type = ErrorType.NOT_FOUND
                task.error_message = "Package not found in repository"
            elif "conflict" in error_str or "depends" in error_str:
                task.error_type = ErrorType.CONFLICT
                task.error_message = "Dependency conflict"
            elif "permission" in error_str or "denied" in error_str:
                task.error_type = ErrorType.PERMISSION
                task.error_message = "Permission denied"
            elif "space" in error_str or "disk" in error_str or "full" in error_str:
                task.error_type = ErrorType.DISK_FULL
                task.error_message = "Insufficient disk space"
            else:
                task.error_type = ErrorType.UNKNOWN
                task.error_message = error_msg

            update_status("error")
            log_msg(f"[red]Error [{task.error_type.value}]:[/] {task.error_message}")
            self.notify(
                f"Error ({task.error_type.value}): {action} {name}", severity="error"
            )
        finally:
            save_task(
                package_name=task.package.name,
                package_source=task.package.source,
                action=task.action,
                status=task.status,
                error_type=task.error_type.value,
                error_message=task.error_message,
            )
            self._running_tasks.pop(task.id, None)
            self._sync_bottom_panel_state()

    async def action_export_packages(self):
        """Export installed packages to JSON/CSV for backup."""
        import csv
        import socket
        from datetime import datetime

        if not self.all_packages:
            self.notify("No packages available to export", severity="warning")
            return

        timestamp = datetime.now()
        date_str = timestamp.strftime("%Y-%m-%d")
        datetime_iso = timestamp.isoformat()
        hostname = socket.gethostname()

        try:
            with open("/etc/os-release") as f:
                os_info = {}
                for line in f:
                    if "=" in line:
                        k, v = line.strip().split("=", 1)
                        os_info[k] = v.strip('"')
            system_name = (
                f"{os_info.get('NAME', 'Unknown')} {os_info.get('VERSION_ID', '')}"
            )
        except (OSError, IOError):
            system_name = "Unknown Linux"

        export_packages = [
            {
                "source": pkg.source,
                "name": pkg.name,
                "version": pkg.version,
                "description": pkg.description,
            }
            for pkg in self.all_packages
        ]

        total_count = len(self.all_packages)
        export_data = {
            "export_date": datetime_iso,
            "system": system_name,
            "hostname": hostname,
            "total_packages": total_count,
            "packages": export_packages,
        }

        home = Path.home()
        downloads_dir = home / "Downloads"
        docs_dir = home / "Documents"
        output_dir = downloads_dir if downloads_dir.exists() else docs_dir

        json_filename = f"linget-backup-{timestamp.strftime('%Y-%m-%d-%H%M%S')}.json"
        json_path = output_dir / json_filename

        try:
            with open(json_path, "w", encoding="utf-8") as f:
                json.dump(export_data, f, indent=2, ensure_ascii=False)
            self.notify(
                f"Exported {total_count} packages to {json_path}",
                severity="information",
            )
        except (OSError, IOError) as e:
            self.notify(f"Failed to export JSON: {e}", severity="error")
            return

        csv_filename = f"linget-backup-{timestamp.strftime('%Y-%m-%d-%H%M%S')}.csv"
        csv_path = output_dir / csv_filename

        try:
            with open(csv_path, "w", newline="", encoding="utf-8") as f:
                writer = csv.writer(f)
                writer.writerow(["source", "name", "version", "export_date"])
                for pkg in self.all_packages:
                    writer.writerow([pkg.source, pkg.name, pkg.version, datetime_iso])
        except (OSError, IOError) as e:
            self.notify(f"Failed to export CSV: {e}", severity="error")
            return

        try:
            log = self.query_one("#term-log", RichLog)
            log.write(f"[green]Exported {total_count} packages:[/]")
            log.write(f"  JSON: {json_path}")
            log.write(f"  CSV: {csv_path}")
            source_counts: Dict[str, int] = {}
            for pkg in export_packages:
                source_counts[pkg["source"]] = source_counts.get(pkg["source"], 0) + 1
            for source, count in sorted(source_counts.items()):
                log.write(f"  {source}: {count} packages")
        except Exception:
            pass

    async def action_import_packages(self):
        """Import packages from backup JSON file."""
        search_paths = [
            Path.home() / "Downloads",
            Path.home() / "Documents",
            Path.home() / ".config" / "linget",
            Path.home(),
        ]

        backup_files: List[Path] = []
        for search_path in search_paths:
            if search_path.exists():
                for ext in ["linget-backup*.json", "linget-backup*.backup", "linget-backup*.linget"]:
                    backup_files.extend(search_path.glob(ext))

        if not backup_files:
            self.notify(
                "No backup files found in Downloads, Documents, or ~/.config/linget/",
                severity="warning",
                timeout=5.0,
            )
            return

        backup_files.sort(key=lambda p: p.stat().st_mtime, reverse=True)

        import_data = None
        selected_file = None

        for backup_file in backup_files[:5]:
            try:
                with open(backup_file, "r") as f:
                    data = json.load(f)

                packages = None
                if isinstance(data, list):
                    packages = data
                elif isinstance(data, dict) and "packages" in data:
                    packages = data["packages"]
                    if isinstance(packages, dict):
                        packages = [
                            {"source": source, **pkg}
                            for source, source_packages in packages.items()
                            for pkg in source_packages
                            if isinstance(pkg, dict)
                        ]

                if packages and len(packages) > 0:
                    import_data = packages
                    selected_file = backup_file
                    break
            except (json.JSONDecodeError, IOError, KeyError):
                continue

        if not import_data or not selected_file:
            self.notify(
                "No valid package backup files found", severity="error", timeout=5.0
            )
            return

        source_counts: Dict[str, int] = {}
        valid_packages: List[Dict[str, str]] = []

        for item in import_data:
            if not isinstance(item, dict):
                continue
            source = item.get("source", "")
            name = item.get("name", "")

            if source and name:
                valid_packages.append(item)
                source_counts[source] = source_counts.get(source, 0) + 1

        if not valid_packages:
            self.notify("No valid packages found in backup file", severity="error")
            return

        try:
            log = self.query_one("#term-log", RichLog)
            log.write(f"[cyan]IMPORT:[/] Found backup: {selected_file}")
            log.write(
                f"[cyan]IMPORT:[/] {', '.join(f'{count} {source.upper()}' for source, count in sorted(source_counts.items()))}"
            )
        except Exception:
            pass

        installed_set = {f"{p.source}-{p.name}" for p in self.all_packages}

        to_install: List[Dict[str, str]] = []
        already_installed: List[Dict[str, str]] = []

        for item in valid_packages:
            row_key = f"{item['source']}-{item['name']}"
            if row_key in installed_set:
                already_installed.append(item)
            else:
                to_install.append(item)

        try:
            log = self.query_one("#term-log", RichLog)
            log.write("[bold]Import Preview:[/]")
            log.write(f"  Total packages in backup: {len(valid_packages)}")
            log.write(f"  Already installed: {len(already_installed)}")
            log.write(f"  Ready to install: {len(to_install)}")
            if to_install:
                for source, count in sorted(source_counts.items()):
                    log.write(f"    - {source.upper()}: {count}")
        except Exception:
            pass

        if not to_install:
            self.notify(
                f"All {len(already_installed)} packages already installed",
                severity="information",
            )
            return

        if hasattr(self, "_pending_import") and self._pending_import == selected_file:
            self._pending_import = None
        else:
            self._pending_import = selected_file
            self.notify(
                f"Import {len(to_install)} packages? Press Ctrl+I again to confirm",
                severity="information",
                timeout=5.0,
            )
            return

        queued_count = 0
        for item in to_install:
            pkg = Package(
                name=item["name"],
                version=item.get("version", "?"),
                source=item["source"],
                status=PackageStatus.NOT_INSTALLED,
                desc=item.get("description", "Imported from backup"),
            )

            task = Task(pkg, "install")
            self.tasks.append(task)
            self.query_one("#queue-panel", QueuePanel).add_task(task)
            self._sync_bottom_panel_state()
            asyncio.ensure_future(self.run_task(task))
            queued_count += 1

        self.notify(
            f"Importing {queued_count} packages from backup...", severity="information"
        )
        try:
            log = self.query_one("#term-log", RichLog)
            log.write(
                f"[green]Import queued:[/] {queued_count} packages ready for installation"
            )
        except Exception:
            pass


if __name__ == "__main__":
    app = LinGetApp()
    app.run()
