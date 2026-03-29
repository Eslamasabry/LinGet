#!/usr/bin/env python3
"""
LinGet - The Universal Package Manager
A rich, perfect, and fancy TUI for managing system packages.
"""

from textual.app import App, ComposeResult
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
    OptionList,
    TabbedContent,
    TabPane,
    RichLog,
    Markdown,
    Tabs,
    Tab,
    Switch,
)
from textual.containers import Horizontal, Vertical, VerticalScroll, Container
from textual.reactive import reactive
from textual.binding import Binding
from textual import work
from textual.widgets.option_list import Option
from textual.command import CommandPalette

import asyncio
import re
import json
import sys
import shutil
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
    load_cached_packages,
    save_cached_packages,
    is_cache_valid,
    get_cache_timestamp,
    get_cache_age_text,
    should_use_cache,
)
from linget import logger, validation

# Setup logging
logger.setup_logging(level=logger.logging.INFO)

# Constants
MAX_CONCURRENT_TASKS = 50
DEFAULT_TIMEOUT = 300


# --- Custom Widgets ---


class PackageTable(DataTable):
    """Custom DataTable widget for displaying package information with bulk selection support."""

    def on_mount(self):
        """Initialize the table with columns and styling."""
        self.cursor_type = "row"
        self.add_columns("☐", "Status", "Name", "Version", "Source", "Size")
        self.zebra_stripes = True
        self.selected_rows: Set[str] = set()

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

        for pkg in packages:
            status_render = {
                PackageStatus.INSTALLED: "[bold green]✅[/] Installed",
                PackageStatus.UPDATE: "[bold yellow]🔄[/] Update",
                PackageStatus.NOT_INSTALLED: "[bold dim]📥[/] Available",
            }.get(pkg.status, "[dim]?[/] Unknown")

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

            row_key = f"{pkg.source}-{pkg.name}"
            checkbox = "☑" if row_key in self.selected_rows else "☐"

            try:
                self.add_row(
                    checkbox,
                    status_render,
                    f"[bold]{pkg.name}[/]",
                    pkg.version,
                    f"[bold {source_color}]{source_logo}[/]",
                    pkg.size or "-",
                    key=row_key,
                )
            except (KeyError, ValueError, TypeError) as e:
                logger.error(f"Failed to add row for {pkg.name}: {e}")


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

        status_color = {
            PackageStatus.INSTALLED: "green",
            PackageStatus.UPDATE: "yellow",
            PackageStatus.NOT_INSTALLED: "dim",
        }.get(p.status, "white")

        status_text = {
            PackageStatus.INSTALLED: "Currently Installed",
            PackageStatus.UPDATE: "Update Available",
            PackageStatus.NOT_INSTALLED: "Not Installed",
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

        return f"""
# {fav_icon}📦 {p.name}
**🏷️ Version:** `{p.version}`
**🏢 Source:** `{source_logo}`
**📏 Size:** `{p.size or "Unknown"}`

[{status_color} bold]● {status_text}[/]

---
**📝 Description:**
{p.description or "No description provided by the package manager."}

---
**⚡ Actions:**
- Press `i` to **Install**
- Press `u` to **Update**
- Press `r` to **Remove**
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

        self.mount(Markdown(self.render_info()))


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
        yield Label("Queued", id=f"status-{self.task_data.id}", classes="task-status")

    def update_progress(self, progress: float, status: str) -> None:
        """Update the task progress display.

        Args:
            progress: Progress percentage (0-100)
            status: Current status string (running, done, error)
        """
        self.progress_bar.progress = progress
        try:
            status_label = self.query_one(f"#status-{self.task_data.id}", Label)
            if status == "running":
                status_label.update("[cyan]Running...[/]")
            elif status == "done":
                status_label.update("[bold green]Complete[/]")
            elif status == "error":
                status_label.update("[bold red]Failed[/]")
        except Exception:
            pass


class QueuePanel(VerticalScroll):
    """Panel for displaying the task queue."""

    def compose(self) -> ComposeResult:
        """Compose the queue panel UI."""
        yield Label("No active tasks.", id="empty-queue", classes="dim")

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

        row = TaskRow(task, id=f"task-row-{task.id}")
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
        asyncio.ensure_future(self.app.action_show_dependencies())
        self.dismiss()

    def action_show_versions(self):
        asyncio.ensure_future(self.app.action_show_versions())
        self.dismiss()

    def action_show_orphans(self):
        asyncio.ensure_future(self.app.action_show_orphans())
        self.dismiss()

    def action_toggle_favorite(self):
        self.app.action_toggle_favorite()
        self.dismiss()

    def action_clean_cache(self):
        asyncio.ensure_future(self.app.action_clean_cache())
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
    }

    #main-layout { height: 1fr; }

    #content-row {
        height: 1fr;
        layout: horizontal;
    }

    #sidebar {
        width: 25;
        dock: left;
        border-right: solid $panel;
        background: $surface;
    }

    #content-area {
        height: 1fr;
        width: 1fr;
    }

    #toolbar {
        height: 3;
        padding: 0 1;
        background: $surface;
        border-bottom: solid $panel;
        align-vertical: middle;
    }

    #search {
        width: 1fr;
        margin-right: 1;
    }

    #split-view { height: 1fr; layout: horizontal; }
    #table-container { width: 2fr; height: 1fr; }

    #info-panel {
        width: 1fr;
        height: 1fr;
        border-left: solid $panel;
        background: $surface-darken-1;
        padding: 1 2;
    }

    Markdown {
        margin: 0 1;
    }

    #bottom-panel {
        height: 12;
        dock: bottom;
        border-top: solid $panel;
        background: $surface;
    }

    .status-bar {
        height: 1;
        background: $surface;
        align-vertical: middle;
    }

    .status-toggles {
        width: auto;
        align-horizontal: right;
        align-vertical: middle;
    }

    .status-label {
        margin: 0 1;
        text-style: bold;
    }

    Switch {
        margin: 0 2 0 0;
    }

    .sidebar-title {
        padding: 1 2;
        text-style: bold;
        color: $accent;
        border-bottom: solid $panel;
    }

    .panel-title {
        padding: 0 1;
        background: $accent;
        color: $text;
        text-style: bold;
        width: 100%;
    }

    #source-list { border: none; background: transparent; }
    PackageTable { height: 1fr; border: none; }

    TaskRow { height: 1; margin: 0 1; }
    .task-label { width: 20; }
    ProgressBar { width: 1fr; margin: 0 2; }
    .task-status { width: 12; text-align: right; }
    .empty-info { margin-top: 2; text-align: center; width: 100%; }

    #loading-overlay {
        width: 100%;
        height: 100%;
        background: $background 50%;
        align: center middle;
        layer: overlay;
        display: none;
    }
    #loading-overlay.-active { display: block; }
    #loading-msg { text-align: center; margin-top: 1; text-style: bold; color: $accent; }
    """

    BINDINGS = [
        Binding("q", "quit", "Quit", show=True),
        Binding("i", "install", "Install", show=True),
        Binding("u", "update", "Update", show=True),
        Binding("r", "remove", "Remove", show=True),
        Binding("R", "retry_task", "Retry", show=True),
        Binding("d", "show_dependencies", "Deps", show=True),
        Binding("space", "toggle_select", "Select", show=True),
        Binding("a", "select_all", "Select All", show=True),
        Binding("A", "deselect_all", "Deselect All", show=True),
        Binding("I", "bulk_install", "Bulk Install", show=True),
        Binding("U", "bulk_update", "Bulk Update", show=True),
        Binding("z", "undo", "Undo", show=True),
        Binding("f", "toggle_favorite", "Favorite", show=True),
        Binding("o", "show_orphans", "Orphans", show=True),
        Binding("X", "clean_cache", "Clean Cache", show=True),
        Binding("v", "show_versions", "Versions", show=True),
        Binding("escape", "cancel_task", "Cancel", show=True),
        Binding("/", "focus_search", "Search", show=True),
        Binding("ctrl+r", "refresh_data", "Refresh", show=True),
        Binding("ctrl+shift+r", "force_refresh", "Force Refresh", show=True),
        Binding("ctrl+e", "export_packages", "Export", show=True),
        Binding("c", "clear_tasks", "Clear Queue", show=True),
        Binding("ctrl+i", "import_packages", "Import", show=True),
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

    def compose(self) -> ComposeResult:
        """Compose the main application UI."""
        yield Header(show_clock=True, icon="📦")

        with Vertical(id="main-layout"):
            with Horizontal(id="content-row"):
                with Vertical(id="sidebar"):
                    yield Label("Sources", classes="sidebar-title")
                    source_list = OptionList(
                        Option("🌍 All Sources", id="all"),
                        Option("⭐ Favorites", id="favorites"),
                        Option(" APT Packages", id="apt"),
                        Option("󰏖 Flatpak Apps", id="flatpak"),
                        Option("📦 Snap Packages", id="snap"),
                        Option("🗼 AUR Packages", id="aur"),
                        Option("🎩 DNF Packages", id="dnf"),
                        Option("🍺 Homebrew", id="brew"),
                        Option(" Cargo Crates", id="cargo"),
                        Option(" NPM Packages", id="npm"),
                        Option(" PIP Packages", id="pip"),
                        id="source-list",
                    )
                    yield source_list

                with Vertical(id="content-area"):
                    yield Tabs(
                        Tab("📦 All Packages", id="mode-all"),
                        Tab("⬆️ Has Updates", id="mode-updates"),
                        Tab("🔍 Search for New", id="mode-search"),
                        id="mode-tabs",
                    )
                    with Horizontal(id="toolbar"):
                        yield Input(
                            placeholder="🔍 Search current view... (/ to focus)",
                            id="search",
                        )
                        yield Button("↻ Refresh", id="refresh-btn", variant="primary")

                    with Horizontal(id="split-view"):
                        with Container(id="table-container"):
                            yield PackageTable(id="package-table")
                        yield InfoPanel(id="info-panel")

            with Vertical(id="bottom-panel"):
                with Horizontal(classes="status-bar"):
                    yield Label("⚙️ Task Queue & Output", classes="panel-title")
                    with Horizontal(classes="status-toggles"):
                        yield Label("Offline:", classes="status-label")
                        yield Switch(id="offline-toggle", value=False)
                        yield Label("Auto-refresh:", classes="status-label")
                        yield Switch(id="auto-refresh-toggle", value=True)
                with TabbedContent():
                    with TabPane("📋 Tasks"):
                        yield QueuePanel(id="queue-panel")
                    with TabPane("💻 Terminal Log"):
                        yield RichLog(id="term-log", highlight=True, max_lines=1000)

        with Vertical(id="loading-overlay"):
            yield LoadingIndicator()
            yield Label("Initializing...", id="loading-msg")

        yield Footer()

    def on_mount(self):
        """Initialize the application on mount."""
        self.title = "LinGet - Universal Package Manager"

        self._settings = load_settings()
        self.theme = self._settings.get("theme", "monokai")
        self._offline_mode = self._settings.get("offline_mode", False)
        self.current_source = self._settings.get("default_source", "all")

        self._is_macos = sys.platform == "darwin"
        self._has_brew = False
        if self._is_macos:
            self._has_brew = shutil.which("brew") is not None

        self.favorites = load_favorites()

        cached_packages = load_cached_packages()
        if cached_packages:
            self.all_packages = sorted(cached_packages, key=lambda p: p.name.lower())
            self._update_package_table()
            self.notify(
                f"Loaded {len(self.all_packages)} packages from cache",
                severity="information",
                timeout=2,
            )
            self._cache_refresh_in_progress = True
            asyncio.ensure_future(self._background_fetch())
        else:
            self.action_refresh_data()

        self._plugin_registry = get_plugin_registry()
        plugin_count = load_plugins(self._plugin_registry)
        if plugin_count > 0:
            self._add_plugin_sources_to_sidebar()
            term_log = self.query_one("#term-log", RichLog)
            term_log.write_line(f"[cyan]Plugins:[/] Loaded {plugin_count} plugin(s)")
            for error in self._plugin_registry.load_errors:
                term_log.write_line(f"[yellow]Plugin warning:[/] {error}")

        # Apply settings after UI is ready
        self.call_later(self._apply_settings_to_ui)

        if self._settings.get("auto_refresh", True):
            self.set_interval(
                self._settings.get("refresh_interval", 600), self._background_refresh
            )

        self.set_interval(30, self._check_network)

    def _apply_settings_to_ui(self):
        """Apply loaded settings to UI widgets."""
        try:
            offline_switch = self.query_one("#offline-toggle", Switch)
            offline_switch.value = self._offline_mode
        except Exception:
            pass

        try:
            auto_refresh_switch = self.query_one("#auto-refresh-toggle", Switch)
            auto_refresh_switch.value = self._settings.get("auto_refresh", True)
        except Exception:
            pass

        try:
            source_list = self.query_one("#source-list", OptionList)
            source_list.highlighted = self._get_source_index(self.current_source)
        except Exception:
            pass

    def _get_source_index(self, source_id: str) -> int:
        """Convert source ID to OptionList index.

        Args:
            source_id: Source identifier string

        Returns:
            Index in the OptionList or 0 if not found
        """
        source_order = [
            "all",
            "favorites",
            "apt",
            "flatpak",
            "snap",
            "aur",
            "dnf",
            "brew",
            "cargo",
            "npm",
            "pip",
        ]
        try:
            return source_order.index(source_id)
        except ValueError:
            return 0

    def _add_plugin_sources_to_sidebar(self):
        """Add plugin sources to the sidebar dynamically."""
        if not self._plugin_registry:
            return

        try:
            source_list = self.query_one("#source-list", OptionList)
            for plugin in self._plugin_registry.plugins:
                source_list.add_option(
                    Option(f"🔌 {plugin.name.title()}", id=plugin.name)
                )
        except Exception as e:
            logger.error(f"Plugin sidebar error: {e}")

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
            table.populate(self._get_filtered_packages())
        except Exception:
            pass

    def _get_filtered_packages(self) -> List[Package]:
        """Get filtered packages based on current source/mode.

        Returns:
            List of packages matching current filters
        """
        if not self.all_packages:
            return []

        if self.current_source == "all":
            return self.all_packages
        elif self.current_source == "favorites":
            return [p for p in self.all_packages if p.row_key in self.favorites]
        else:
            return [p for p in self.all_packages if p.source == self.current_source]

    def _check_network(self):
        """Check network connectivity."""
        try:
            import urllib.request

            urllib.request.urlopen("https://pypi.org", timeout=3)
            if self._offline_mode:
                self._offline_mode = False
                self.notify("Back online", severity="information")
        except (urllib.error.URLError, OSError, TimeoutError):
            if not self._offline_mode:
                self._offline_mode = True
                self.notify(
                    "Offline mode - remote operations disabled", severity="warning"
                )

    async def fetch_packages(self):
        """Asynchronously fetch packages without blocking event loop."""
        packages: List[Package] = []

        def log_msg(msg: str):
            try:
                self.query_one("#loading-msg", Label).update(msg)
            except Exception:
                pass
            try:
                self.query_one("#term-log", RichLog).write_line(f"[cyan]INFO:[/] {msg}")
            except Exception:
                pass

        async def run_cmd(cmd: List[str]) -> Tuple[int, str]:
            proc = await asyncio.create_subprocess_exec(
                *cmd, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
            )
            stdout, _ = await proc.communicate()
            return proc.returncode, stdout.decode(errors="ignore")

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
            logger.log_exception(f"APT error: {e}")
            log_msg(f"APT error: {e}")

        log_msg("Fetching Flatpak packages...")
        try:
            code, out = await run_cmd(["flatpak", "list", "--app"])
            if code == 0:
                for line in out.splitlines():
                    parts = line.split("\t")
                    if len(parts) >= 3:
                        packages.append(
                            Package(
                                parts[0],
                                parts[1],
                                "flatpak",
                                PackageStatus.INSTALLED,
                                desc="Flatpak Application",
                            )
                        )
        except (OSError, asyncio.TimeoutError, ValueError) as e:
            logger.log_exception(f"Flatpak error: {e}")
            log_msg(f"Flatpak error: {e}")

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
            logger.log_exception(f"Cargo error: {e}")
            log_msg(f"Cargo error: {e}")

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
        except (OSError, asyncio.TimeoutError, json.JSONDecodeError, ValueError) as e:
            logger.log_exception(f"NPM error: {e}")
            log_msg(f"NPM error: {e}")

        log_msg("Fetching PIP packages...")
        try:
            code, out = await run_cmd(["pip", "list", "--format=json"])
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
        except (OSError, asyncio.TimeoutError, json.JSONDecodeError, ValueError) as e:
            logger.log_exception(f"PIP error: {e}")
            log_msg(f"PIP error: {e}")

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
        except (OSError, asyncio.TimeoutError, ValueError) as e:
            logger.log_exception(f"Snap error: {e}")
            log_msg(f"Snap error: {e}")

        log_msg("Fetching AUR packages...")
        try:
            aur_helper = None
            for helper in ["yay", "paru"]:
                proc = await asyncio.create_subprocess_exec(
                    "which",
                    helper,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE,
                )
                await proc.communicate()
                if proc.returncode == 0:
                    aur_helper = helper
                    break

            if aur_helper:
                code, out = await run_cmd([aur_helper, "-Q"])
                if code == 0:
                    code_official, out_official = await run_cmd(["pacman", "-Qn"])
                    official_packages: Set[str] = set()
                    if code_official == 0:
                        for line in out_official.splitlines():
                            parts = line.split()
                            if len(parts) >= 1:
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
            else:
                log_msg("No AUR helper found (yay/paru)")
        except (OSError, asyncio.TimeoutError, ValueError) as e:
            logger.log_exception(f"AUR error: {e}")
            log_msg(f"AUR error: {e}")

        log_msg("Fetching DNF packages...")
        try:
            proc = await asyncio.create_subprocess_exec(
                "which",
                "dnf",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            await proc.communicate()
            if proc.returncode == 0:
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
                if code == 0 or code == 100:
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
            logger.log_exception(f"DNF error: {e}")
            log_msg(f"DNF error: {e}")

        if self._is_macos and self._has_brew:
            log_msg("Fetching Homebrew packages...")
            try:
                code, out = await run_cmd(["brew", "list", "--versions", "--formula"])
                if code == 0:
                    for line in out.splitlines():
                        parts = line.split()
                        if len(parts) >= 2:
                            name = parts[0]
                            version = parts[1]
                            packages.append(
                                Package(
                                    name,
                                    version,
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
                            name = parts[0]
                            version = parts[1]
                            packages.append(
                                Package(
                                    name,
                                    version,
                                    "brew",
                                    PackageStatus.INSTALLED,
                                    desc="Homebrew Cask",
                                )
                            )

                code, out = await run_cmd(["brew", "outdated", "--quiet"])
                if code == 0:
                    outdated_names = set(
                        line.split()[0] for line in out.splitlines() if line.strip()
                    )
                    for pkg in packages:
                        if pkg.source == "brew" and pkg.name in outdated_names:
                            pkg.status = PackageStatus.UPDATE
            except (OSError, asyncio.TimeoutError, ValueError) as e:
                logger.log_exception(f"Homebrew error: {e}")
                log_msg(f"Homebrew error: {e}")

        if self._plugin_registry:
            log_msg("Fetching plugin packages...")
            try:
                plugin_packages = self._plugin_registry.get_all_installed()
                if plugin_packages:
                    packages.extend(plugin_packages)
                    log_msg(f"Found {len(plugin_packages)} packages from plugins")
            except Exception as e:
                logger.log_exception(f"Plugin error: {e}")

        self.all_packages = sorted(packages, key=lambda p: p.name.lower())
        save_cached_packages(self.all_packages)

        try:
            self.query_one("#loading-overlay").display = False
        except Exception:
            pass
        self.apply_filters()
        self._cache_refresh_in_progress = False
        log_msg("Refresh complete.")

    async def search_new_packages(self, query: str):
        """Search for new packages across repositories.

        Args:
            query: Search query string
        """
        if not validation.is_valid_search_query(query):
            self.notify("Invalid search query", severity="warning")
            return

        from linget.search import search_new_packages as do_search

        def log_msg(msg: str):
            try:
                self.query_one("#loading-msg", Label).update(msg)
            except Exception:
                pass
            try:
                self.query_one("#term-log", RichLog).write_line(
                    f"[cyan]SEARCH:[/] {msg}"
                )
            except Exception:
                pass

        log_msg(f"Searching for '{query}'...")
        found_packages = await do_search(query, self.all_packages, self.current_source)

        if self._plugin_registry and (
            self.current_source == "all"
            or self._plugin_registry.get(self.current_source)
        ):
            log_msg("Searching plugins...")
            try:
                plugin_results = self._plugin_registry.search_all(query)
                if plugin_results:
                    log_msg(f"Found {len(plugin_results)} packages from plugins")
                    found_packages.extend(plugin_results)
            except Exception as e:
                logger.log_exception(f"Plugin search error: {e}")

        if found_packages:
            log_msg(f"Found {len(found_packages)} new packages")
            self.all_packages = [
                p
                for p in self.all_packages
                if not (
                    p.status == PackageStatus.NOT_INSTALLED
                    and (
                        self.current_source == "all" or p.source == self.current_source
                    )
                )
            ]
            self.all_packages.extend(found_packages)
            self.all_packages = sorted(self.all_packages, key=lambda p: p.name.lower())
        else:
            log_msg("No new packages found")

        self.apply_filters()

    def apply_filters(self):
        """Apply current filters to package list and update table."""
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
                p for p in filtered if q in p.name.lower() or q in p.description.lower()
            ]

        table = self.query_one("#package-table", PackageTable)
        table.populate(filtered, self.favorites)

        if filtered:
            table.move_cursor(row=0)
            self.update_info_panel(filtered[0])
        else:
            self.query_one("#info-panel", InfoPanel).package = None
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
        """Update the info panel with package details.

        Args:
            package: Package to display or None
        """
        info_panel = self.query_one("#info-panel", InfoPanel)
        info_panel.package = package
        if package:
            for child in list(info_panel.children):
                child.remove()
            info_panel.mount(Markdown(info_panel.render_info(self.favorites)))

    def on_button_pressed(self, event: Button.Pressed) -> None:
        """Handle button press events.

        Args:
            event: Button pressed event
        """
        if event.button.id == "refresh-btn":
            self.action_refresh_data()

    def on_switch_changed(self, event: Switch.Changed) -> None:
        """Handle switch toggle events.

        Args:
            event: Switch changed event
        """
        if event.switch.id == "offline-toggle":
            self._offline_mode = event.value
            self._settings["offline_mode"] = event.value
            save_settings(self._settings)
            if self._offline_mode:
                self.notify("Offline mode enabled", severity="warning")
            else:
                self.notify("Online mode restored", severity="information")
        elif event.switch.id == "auto-refresh-toggle":
            self._settings["auto_refresh"] = event.value
            save_settings(self._settings)
            if event.value:
                self.set_interval(600, self._background_refresh)
                self.notify("Auto-refresh enabled (10 min)", severity="information")
            else:
                self.notify("Auto-refresh disabled", severity="warning")

    def on_input_changed(self, event: Input.Changed) -> None:
        """Handle input change events.

        Args:
            event: Input changed event
        """
        if event.input.id == "search":
            self.search_query = event.value
            if self.current_mode == "mode-search" and len(self.search_query) >= 2:
                asyncio.ensure_future(self.search_new_packages(self.search_query))
            else:
                self.apply_filters()

    def on_option_list_option_selected(self, event: OptionList.OptionSelected) -> None:
        """Handle source selection events.

        Args:
            event: Option selected event
        """
        valid_sources = {
            "all",
            "apt",
            "flatpak",
            "snap",
            "cargo",
            "npm",
            "pip",
            "favorites",
            "aur",
            "dnf",
            "brew",
        }
        if event.option.id in valid_sources:
            self.current_source = event.option.id
            self._settings["default_source"] = event.option.id
            save_settings(self._settings)
            self.apply_filters()

    def on_tabs_tab_activated(self, event: Tabs.TabActivated) -> None:
        """Handle tab activation events.

        Args:
            event: Tab activated event
        """
        if event.tab.id in ("mode-all", "mode-updates", "mode-search"):
            self.current_mode = event.tab.id
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
        try:
            self.query_one("#loading-overlay").display = True
        except Exception:
            pass
        asyncio.ensure_future(self.fetch_packages())

    def action_force_refresh(self):
        """Force refresh package data ignoring cache."""
        self._force_refresh = True
        try:
            self.query_one("#loading-overlay").display = True
        except Exception:
            pass
        asyncio.ensure_future(self.fetch_packages())

    def _queue_task(self, action: str):
        """Queue a task for the selected package.

        Args:
            action: Action to perform (install, update, remove)
        """
        info_panel = self.query_one("#info-panel", InfoPanel)
        pkg = info_panel.package
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

        try:
            term = self.query_one("#term-log", RichLog)
            term.write_line(f"[yellow]QUEUED:[/] {action.upper()} {pkg.name}")
        except Exception:
            pass
        self.notify(f"Queued: {action} {pkg.name}", severity="information")

        asyncio.ensure_future(self.run_task(task))

    def action_install(self):
        """Queue install action for selected package."""
        self._queue_task("install")

    def action_update(self):
        """Queue update action for selected package."""
        self._queue_task("update")

    def action_remove(self):
        """Queue remove action for selected package with confirmation."""
        info_panel = self.query_one("#info-panel", InfoPanel)
        pkg = info_panel.package
        if not pkg:
            self.notify("No package selected!", severity="warning")
            return

        self.notify(
            f"Press 'r' again to confirm removing {pkg.name}",
            severity="error",
            timeout=3.0,
        )

        if hasattr(self, "_pending_remove") and self._pending_remove == pkg:
            self._pending_remove = None
            self._queue_task("remove")
        else:
            self._pending_remove = pkg

    def action_clear_tasks(self):
        """Clear completed tasks from queue."""
        to_remove = [t for t in self.tasks if t.status in ("done", "error")]
        for t in to_remove:
            try:
                row = self.query_one(f"#task-row-{t.id}")
                row.remove()
            except Exception:
                pass

        queue_panel = self.query_one("#queue-panel", QueuePanel)
        existing = queue_panel.query("#empty-queue")
        if not existing and not [
            t for t in self.tasks if t.status not in ("done", "error")
        ]:
            queue_panel.mount(
                Label("No active tasks.", id="empty-queue", classes="dim")
            )

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
                running_task.error_type = ErrorType.AUTH_CANCELLED
                self.notify(f"Cancelled: {running_task.package.name}")
                try:
                    self.query_one("#term-log", RichLog).write_line(
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

        try:
            self.query_one("#term-log", RichLog).write_line(
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
        info_panel = self.query_one("#info-panel", InfoPanel)
        pkg = info_panel.package
        if not pkg:
            self.notify("No package selected!", severity="warning")
            return

        try:
            self.query_one("#term-log", RichLog).write_line(
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
                logger.log_exception(f"Error fetching deps: {e}")
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
            log.write_line(f"[bold]Dependencies for {pkg.name}:[/]")
            if deps:
                for dep in deps[:10]:
                    log.write_line(f"  • {dep}")
                if len(deps) > 10:
                    log.write_line(f"  ... and {len(deps) - 10} more")
            else:
                log.write_line("  No dependencies found")

            if reverse_deps:
                log.write_line(f"\n[bold]Required by:[/]")
                for rdep in reverse_deps[:5]:
                    log.write_line(f"  • {rdep}")
                if len(reverse_deps) > 5:
                    log.write_line(f"  ... and {len(reverse_deps) - 5} more")
        except Exception:
            pass

        self.notify(f"Dependencies shown for {pkg.name}")

    def action_toggle_select(self):
        """Toggle selection of current package."""
        table = self.query_one("#package-table", PackageTable)
        if not table.cursor_row:
            return

        row_key = table.cursor_row.value
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
            self.notify(
                f"Bulk {action}: {len(packages)} packages. Press {action.upper()} again to confirm.",
                severity="warning",
                timeout=5.0,
            )
            attr_name = f"_pending_bulk_{action}"
            if hasattr(self, attr_name) and getattr(self, attr_name) == packages:
                setattr(self, attr_name, None)
                return True
            setattr(self, attr_name, packages)
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

        try:
            term = self.query_one("#term-log", RichLog)
            term.write_line(f"[yellow]UNDO:[/] {reverse_action.upper()} {pkg.name}")
        except Exception:
            pass
        self.notify(f"Undoing: {reverse_action} {pkg.name}", severity="information")

        asyncio.ensure_future(self.run_task(task))
        self._last_action = None

    def action_toggle_favorite(self):
        """Toggle favorite status for the currently selected package."""
        info_panel = self.query_one("#info-panel", InfoPanel)
        pkg = info_panel.package
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
                        log.write_line(
                            f"[bold yellow]Orphan packages ({len(orphans)}):[/]"
                        )
                        for orphan in orphans[:20]:
                            log.write_line(f"  • {orphan}")
                        if len(orphans) > 20:
                            log.write_line(f"  ... and {len(orphans) - 20} more")

                        self.notify(
                            f"Found {len(orphans)} orphan packages. Run 'sudo apt autoremove' to clean up.",
                            severity="warning",
                            timeout=10.0,
                        )
                    else:
                        log.write_line("[green]No orphan packages found[/]")
                        self.notify("No orphan packages found", severity="information")
                except Exception:
                    pass
            else:
                self.notify("Failed to check for orphans", severity="error")

        except (OSError, asyncio.TimeoutError, ValueError) as e:
            logger.log_exception(f"Error checking orphans: {e}")
            self.notify(f"Error checking orphans: {e}", severity="error")

    async def action_clean_cache(self):
        """Clean package manager cache for the current source."""
        if self.current_source == "all":
            self.notify("Select a specific source to clean cache", severity="warning")
            return

        cache_configs = {
            "apt": [["pkexec", "apt-get", "clean"], ["pkexec", "apt-get", "autoclean"]],
            "flatpak": [["pkexec", "flatpak", "uninstall", "--unused", "-y"]],
            "cargo": [["cargo", "cache", "--autoclean"]],
            "npm": [["npm", "cache", "clean", "--force"]],
            "pip": [["pip", "cache", "purge"]],
            "aur": [["yay", "-Sc", "--noconfirm"]],
            "dnf": [["pkexec", "dnf", "clean", "all"]],
        }

        if self.current_source not in cache_configs:
            self.notify(
                f"Cache cleaning not supported for {self.current_source}",
                severity="warning",
            )
            return

        if (
            hasattr(self, "_pending_clean_cache")
            and self._pending_clean_cache == self.current_source
        ):
            self._pending_clean_cache = None
        else:
            self.notify(
                f"Press 'X' again to confirm cleaning {self.current_source} cache",
                severity="warning",
                timeout=3.0,
            )
            self._pending_clean_cache = self.current_source
            return

        try:
            log = self.query_one("#term-log", RichLog)
            log.write_line(f"[cyan]Cleaning {self.current_source} cache...[/]")
        except Exception:
            pass

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
                    try:
                        log.write_line(f"[dim]Cache size before: {before_size}[/]")
                    except Exception:
                        pass
            except (OSError, ValueError):
                pass

        commands = cache_configs[self.current_source]
        for cmd in commands:
            try:
                proc = await asyncio.create_subprocess_exec(
                    *cmd,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.STDOUT,
                )
                stdout, _ = await proc.communicate()
                if proc.returncode == 0:
                    output = stdout.decode().strip()
                    if output:
                        try:
                            for line in output.splitlines()[:20]:
                                log.write_line(f"  {line}")
                        except Exception:
                            pass
                else:
                    try:
                        log.write_line(f"[red]Command failed: {' '.join(cmd)}[/]")
                    except Exception:
                        pass
            except (OSError, asyncio.TimeoutError, ValueError) as e:
                try:
                    log.write_line(f"[red]Error running {' '.join(cmd)}: {e}[/]")
                except Exception:
                    pass

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
                    try:
                        log.write_line(f"[dim]Cache size after: {after_size}[/]")
                    except Exception:
                        pass
                    self.notify(
                        f"APT cache cleaned (was: {before_size}, now: {after_size})",
                        severity="information",
                        timeout=5.0,
                    )
                    return
            except (OSError, ValueError):
                pass

        self.notify(f"{self.current_source} cache cleaned", severity="information")

    async def action_show_versions(self):
        """Show package version history using apt-cache policy."""
        try:
            log = self.query_one("#term-log", RichLog)
        except Exception:
            return

        try:
            info_panel = self.query_one("#info-panel", InfoPanel)
            package = info_panel.package

            if not package:
                self.notify("No package selected", severity="warning")
                return

            if package.source != "apt":
                self.notify(
                    "Version history only available for apt packages",
                    severity="warning",
                )
                return

            log.write_line(f"[cyan]Fetching version history for {package.name}...[/]")

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

                log.write_line(f"[bold green]Version history for {package.name}:[/]")

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
                    log.write_line(f"  Installed: {installed}")
                else:
                    log.write_line("  Installed: (none)")

                if candidate and candidate != "(none)":
                    log.write_line(f"  Candidate: {candidate}")
                else:
                    log.write_line("  Candidate: (none)")

                if available:
                    log.write_line("  Available:")
                    for version, repo, is_installed in available:
                        if is_installed:
                            log.write_line(f"    • {version} (installed)")
                        else:
                            display_repo = repo.strip("()") if repo else "unknown"
                            log.write_line(f"    • {version} ({display_repo})")
                else:
                    log.write_line("  No versions available in repositories")

                self.notify(
                    f"Version history for {package.name} displayed",
                    severity="information",
                )
            else:
                error_msg = stderr.decode("utf-8", errors="replace").strip()
                log.write_line(f"[red]Failed to fetch versions: {error_msg}[/]")
                self.notify(
                    f"Version history unavailable for {package.name}",
                    severity="warning",
                )

        except Exception as e:
            logger.log_exception(f"Error fetching versions: {e}")
            log.write_line(f"[red]Error fetching versions: {e}[/]")
            self.notify(f"Error fetching versions: {e}", severity="error")

    async def run_task(self, task: Task):
        """Execute real package manager commands natively in the async event loop.

        Args:
            task: Task object containing package and action information
        """
        self._running_tasks[task.id] = None

        def log_msg(msg: str):
            try:
                self.query_one("#term-log", RichLog).write_line(
                    f"[{task.package.source}] {msg}"
                )
            except Exception:
                pass

        def update_status(status: str, progress: Optional[float] = None):
            task.status = status
            if progress is not None:
                task.progress = min(progress, 100.0)
            try:
                row = self.query_one(f"#task-row-{task.id}", TaskRow)
                row.update_progress(task.progress, task.status)
            except Exception:
                pass

        update_status("running", 5.0)
        try:
            self.query_one("#term-log", RichLog).write_line(
                f"[green]STARTED:[/] {task.action.upper()} {task.package.name}"
            )
        except Exception:
            pass

        cmd: List[str] = []
        source = task.package.source
        name = task.package.name
        action = task.action

        if source == "apt":
            base = ["pkexec", "apt-get", "-y"]
            if action == "install":
                cmd = base + ["install", name]
            elif action == "remove":
                cmd = base + ["remove", name]
            elif action == "update":
                cmd = base + ["install", "--only-upgrade", name]
        elif source == "flatpak":
            if action == "install":
                cmd = ["flatpak", "install", "-y", name]
            elif action == "remove":
                cmd = ["flatpak", "uninstall", "-y", name]
            elif action == "update":
                cmd = ["flatpak", "update", "-y", name]
        elif source == "cargo":
            if action in ("install", "update"):
                cmd = ["cargo", "install", name]
            elif action == "remove":
                cmd = ["cargo", "uninstall", name]
        elif source == "npm":
            if action in ("install", "update"):
                cmd = ["npm", "install", "-g", name]
            elif action == "remove":
                cmd = ["npm", "uninstall", "-g", name]
        elif source == "pip":
            if action in ("install", "update"):
                cmd = ["pip", "install", "--upgrade", name]
            elif action == "remove":
                cmd = ["pip", "uninstall", "-y", name]
        elif source == "snap":
            if action == "install":
                cmd = ["pkexec", "snap", "install", name]
            elif action == "remove":
                cmd = ["pkexec", "snap", "remove", name]
            elif action == "update":
                cmd = ["pkexec", "snap", "refresh", name]
        elif source == "aur":
            aur_helper = "yay"
            try:
                proc = await asyncio.create_subprocess_exec(
                    "which",
                    "yay",
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE,
                )
                await proc.communicate()
                if proc.returncode != 0:
                    aur_helper = "paru"
            except (OSError, asyncio.TimeoutError):
                aur_helper = "paru"

            if action in ("install", "update"):
                cmd = [aur_helper, "-S", "--noconfirm", name]
            elif action == "remove":
                cmd = [aur_helper, "-R", "--noconfirm", name]
        elif source == "dnf":
            base = ["pkexec", "dnf", "-y"]
            if action == "install":
                cmd = base + ["install", name]
            elif action == "remove":
                cmd = base + ["remove", name]
            elif action == "update":
                cmd = base + ["upgrade", name]
        elif source == "brew":
            if action == "install":
                cmd = ["brew", "install", name]
            elif action == "remove":
                cmd = ["brew", "uninstall", name]
            elif action == "update":
                cmd = ["brew", "upgrade", name]

        if not cmd:
            log_msg(f"[red]Error:[/] Unsupported action/source combination.")
            update_status("error")
            return

        try:
            if source == "apt" and "pkexec" in cmd:
                update_status("running", 10.0)
                log_msg("[dim]Waiting for authentication...[/]")

            process = await asyncio.wait_for(
                asyncio.create_subprocess_exec(
                    *cmd,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.STDOUT,
                ),
                timeout=DEFAULT_TIMEOUT,
            )

            self._running_tasks[task.id] = process

            while True:
                line = await process.stdout.readline()
                if not line:
                    break

                text_line = line.decode("utf-8", errors="replace").strip()
                if text_line:
                    log_msg(text_line)
                    if task.progress < 95:
                        update_status("running", min(task.progress + 1.5, 95.0))

            await process.wait()
            return_code = process.returncode

            if return_code == 0:
                update_status("done", 100.0)
                self.notify(f"Completed: {action} {name}")
                try:
                    self.query_one("#term-log", RichLog).write_line(
                        f"[bold green]COMPLETED:[/] {task.action.upper()} {task.package.name}"
                    )
                except Exception:
                    pass
                self.action_refresh_data()
            else:
                if return_code == 126 or return_code == 127:
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

        packages_by_source: Dict[str, List[Dict[str, str]]] = {}
        for pkg in self.all_packages:
            if pkg.source not in packages_by_source:
                packages_by_source[pkg.source] = []
            packages_by_source[pkg.source].append(
                {"name": pkg.name, "version": pkg.version}
            )

        total_count = len(self.all_packages)
        export_data = {
            "export_date": datetime_iso,
            "system": system_name,
            "hostname": hostname,
            "total_packages": total_count,
            "packages": packages_by_source,
        }

        home = Path.home()
        downloads_dir = home / "Downloads"
        docs_dir = home / "Documents"
        output_dir = downloads_dir if downloads_dir.exists() else docs_dir

        json_filename = f"linget-backup-{date_str}.json"
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

        csv_filename = f"linget-backup-{date_str}.csv"
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
            log.write_line(f"[green]Exported {total_count} packages:[/]")
            log.write_line(f"  JSON: {json_path}")
            log.write_line(f"  CSV: {csv_path}")
            for source, pkgs in sorted(packages_by_source.items()):
                log.write_line(f"  {source}: {len(pkgs)} packages")
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
                for ext in ["*.json", "*.backup", "*.linget"]:
                    backup_files.extend(search_path.glob(ext))

        specific_names = [
            "linget-backup.json",
            "packages.json",
            "favorites.json",
            "task_history.json",
        ]
        for search_path in search_paths:
            for name in specific_names:
                file_path = search_path / name
                if file_path.exists() and file_path not in backup_files:
                    backup_files.append(file_path)

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
            log.write_line(f"[cyan]IMPORT:[/] Found backup: {selected_file}")
            log.write_line(
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
            log.write_line("[bold]Import Preview:[/]")
            log.write_line(f"  Total packages in backup: {len(valid_packages)}")
            log.write_line(f"  Already installed: {len(already_installed)}")
            log.write_line(f"  Ready to install: {len(to_install)}")
            if to_install:
                for source, count in sorted(source_counts.items()):
                    log.write_line(f"    - {source.upper()}: {count}")
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
            asyncio.ensure_future(self.run_task(task))
            queued_count += 1

        self.notify(
            f"Importing {queued_count} packages from backup...", severity="information"
        )
        try:
            log = self.query_one("#term-log", RichLog)
            log.write_line(
                f"[green]Import queued:[/] {queued_count} packages ready for installation"
            )
        except Exception:
            pass


if __name__ == "__main__":
    app = LinGetApp()
    app.run()
