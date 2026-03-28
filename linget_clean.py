#!/usr/bin/env python3
"""
LinGet - Polished winget-style TUI
Clean, functional, feature-rich
"""

from textual.app import App, ComposeResult
from textual.widgets import (
    DataTable,
    Static,
    Footer,
    Header,
    Input,
    Button,
    ProgressBar,
)
from textual.containers import Horizontal, Vertical, Container, Grid
from textual.reactive import reactive
from textual.binding import Binding
from textual import work
import asyncio
from datetime import datetime
from enum import Enum
import random


class PackageStatus(Enum):
    INSTALLED = "installed"
    UPDATE = "update"
    NOT_INSTALLED = "available"


class Package:
    def __init__(self, name, version, source, status, size="", desc=""):
        self.name = name
        self.version = version
        self.source = source
        self.status = status
        self.size = size
        self.description = desc
        self.selected = False


class Task:
    def __init__(self, package, action):
        self.package = package
        self.action = action
        self.progress = 0
        self.status = "queued"  # queued, running, done


class CleanTable(DataTable):
    """Clean table with subtle styling"""

    def on_mount(self):
        self.add_columns("Name", "Version", "Source", "Size", "Status")
        self.cursor_type = "row"
        self.zebra_stripes = False
        self.show_header = True
        self.show_cursor = True

    def add_pkg(self, pkg):
        status_config = {
            PackageStatus.INSTALLED: ("●", "green", "Installed"),
            PackageStatus.UPDATE: ("⬆", "yellow", "Update"),
            PackageStatus.NOT_INSTALLED: ("○", "dim", "Available"),
        }.get(pkg.status, ("○", "dim", "Unknown"))

        icon, color, label = status_config

        self.add_row(
            pkg.name,
            pkg.version,
            pkg.source,
            pkg.size,
            f"[{color}]{icon} {label}[/]",
            key=pkg.name,
        )


class InfoPanel(Static):
    """Rich info panel with metadata"""

    def show(self, pkg):
        if not pkg:
            self.update("")
            return

        status_config = {
            PackageStatus.INSTALLED: ("●", "green", "INSTALLED"),
            PackageStatus.UPDATE: ("⬆", "yellow", "UPDATE AVAILABLE"),
            PackageStatus.NOT_INSTALLED: ("○", "dim", "NOT INSTALLED"),
        }.get(pkg.status, ("○", "dim", "UNKNOWN"))

        icon, color, label = status_config

        content = f"""[bold]{pkg.name}[/] [dim]{pkg.version}[/]

[{color}]{icon} {label}[/]
Source: {pkg.source}
Size: {pkg.size}

[dim]{pkg.description}[/]

[dim]Press [bold green]i[/] to install, [bold yellow]u[/] to update, [bold red]r[/] to remove[/]"""

        self.update(content)


class QueuePanel(Static):
    """Task queue with progress"""

    def update_tasks(self, tasks):
        if not tasks:
            self.update("[dim]No active tasks[/]")
            return

        lines = []
        for task in tasks[-4:]:  # Show last 4
            icon = {"queued": "○", "running": "▶", "done": "✓"}.get(task.status, "○")

            color = {"queued": "dim", "running": "cyan", "done": "green"}.get(
                task.status, "dim"
            )

            bar = "█" * (task.progress // 5) + "░" * (20 - task.progress // 5)
            lines.append(
                f"[{color}]{icon}[/] {task.package:15} [{color}]{bar}[/] {task.progress}%"
            )

        self.update("\n".join(lines))


class StatsBar(Static):
    """Status bar with counters"""

    def update_stats(self, packages, tasks):
        total = len(packages)
        installed = sum(1 for p in packages if p.status == PackageStatus.INSTALLED)
        updates = sum(1 for p in packages if p.status == PackageStatus.UPDATE)
        available = sum(1 for p in packages if p.status == PackageStatus.NOT_INSTALLED)
        active = sum(1 for t in tasks if t.status == "running")

        stats = f"[dim]Total:[/] {total}  [green]● Installed:[/] {installed}  [yellow]⬆ Updates:[/] {updates}  [dim]○ Available:[/] {available}"
        if active > 0:
            stats += f"  [cyan]▶ Running:[/] {active}"

        self.update(stats)


class LinGetApp(App):
    """Polished winget-style package manager"""

    CSS = """
    Screen {
        background: $surface;
        color: $text;
    }
    
    /* Clean header */
    Header {
        background: $surface;
        color: $text;
        border-bottom: solid $primary;
        height: 1;
    }
    
    /* Search bar */
    #search-row {
        height: 3;
        padding: 0 1;
        border-bottom: solid $surface-darken-1;
    }
    
    #search {
        width: 1fr;
        border: none;
        background: $surface-darken-1;
        padding: 0 1;
    }
    
    #search:focus {
        background: $surface-darken-2;
    }
    
    #clear-btn {
        width: 8;
        border: none;
        background: $surface-darken-1;
    }
    
    /* Main layout */
    #main {
        layout: vertical;
        height: 1fr;
    }
    
    /* Top section - table + info */
    #top-section {
        layout: horizontal;
        height: 75%;
    }
    
    #table-container {
        width: 70%;
        height: 100%;
        border-right: solid $surface-darken-1;
    }
    
    DataTable {
        width: 100%;
        height: 100%;
        border: none;
    }
    
    DataTable > .datatable--header {
        background: $surface-darken-1;
        color: $text;
        text-style: bold;
        border-bottom: solid $primary;
    }
    
    DataTable > .datatable--cursor {
        background: $primary-darken-2;
        color: $text;
    }
    
    /* Info panel */
    #info {
        width: 30%;
        padding: 1 2;
        background: $surface;
    }
    
    /* Queue section */
    #queue-section {
        height: 15%;
        border-top: solid $surface-darken-1;
        padding: 0 1;
    }
    
    #queue-title {
        text-style: bold;
        color: $primary;
        margin: 1 0;
    }
    
    /* Stats bar */
    #stats {
        height: 3;
        padding: 0 1;
        background: $surface-darken-1;
        border-top: solid $surface-darken-2;
        content-align: center middle;
    }
    
    /* Footer */
    Footer {
        background: $surface;
        border-top: solid $surface-darken-1;
    }
    
    Footer > .footer--key {
        background: $primary;
        color: $text;
        text-style: bold;
        padding: 0 1;
    }
    """

    BINDINGS = [
        Binding("q", "quit", "Quit", show=True),
        Binding("i", "install", "Install", show=True),
        Binding("u", "update", "Update", show=True),
        Binding("r", "remove", "Remove", show=True),
        Binding("/", "search", "Search", show=True),
        Binding("1", "filter_all", "All", show=True),
        Binding("2", "filter_apt", "Apt", show=True),
        Binding("3", "filter_flatpak", "Flatpak", show=True),
        Binding("4", "filter_cargo", "Cargo", show=True),
        Binding("5", "filter_updates", "Updates", show=True),
        Binding("n", "demo", "Demo", show=True),
        Binding("c", "clear", "Clear", show=True),
    ]

    filter = reactive("all")
    search_query = reactive("")
    packages = []
    tasks = []
    selected = None
    _init = False

    def compose(self) -> ComposeResult:
        yield Header(show_clock=False)

        with Horizontal(id="search-row"):
            yield Input(placeholder="Search packages...", id="search")
            yield Button("Clear", id="clear-btn")

        with Vertical(id="main"):
            with Horizontal(id="top-section"):
                with Container(id="table-container"):
                    yield CleanTable(id="table")

                yield InfoPanel(id="info")

            with Container(id="queue-section"):
                yield Static("Task Queue", id="queue-title")
                yield QueuePanel(id="queue")

        yield StatsBar(id="stats")
        yield Footer()

    def on_mount(self):
        self.title = "LinGet"
        self.sub_title = "Package Manager"

        # Rich sample data
        self.packages = [
            Package(
                "neovim",
                "0.9.5",
                "apt",
                PackageStatus.INSTALLED,
                "15MB",
                "Vim-based text editor",
            ),
            Package(
                "firefox",
                "124.0.1",
                "flatpak",
                PackageStatus.UPDATE,
                "120MB",
                "Mozilla Firefox web browser",
            ),
            Package(
                "vscode",
                "1.87.2",
                "flatpak",
                PackageStatus.INSTALLED,
                "320MB",
                "Visual Studio Code",
            ),
            Package(
                "docker",
                "25.0.4",
                "apt",
                PackageStatus.UPDATE,
                "150MB",
                "Container runtime platform",
            ),
            Package(
                "nodejs",
                "20.11.1",
                "apt",
                PackageStatus.NOT_INSTALLED,
                "35MB",
                "JavaScript runtime",
            ),
            Package(
                "rust",
                "1.77.0",
                "cargo",
                PackageStatus.INSTALLED,
                "800MB",
                "Rust programming language",
            ),
            Package(
                "alacritty",
                "0.13.1",
                "cargo",
                PackageStatus.INSTALLED,
                "8MB",
                "GPU-accelerated terminal",
            ),
            Package(
                "postgresql",
                "16.2",
                "apt",
                PackageStatus.UPDATE,
                "45MB",
                "Advanced SQL database",
            ),
            Package(
                "ripgrep",
                "14.1.0",
                "cargo",
                PackageStatus.NOT_INSTALLED,
                "5MB",
                "Fast grep alternative",
            ),
            Package(
                "obsidian",
                "1.5.3",
                "flatpak",
                PackageStatus.INSTALLED,
                "180MB",
                "Note-taking app",
            ),
            Package(
                "telegram",
                "4.15.2",
                "flatpak",
                PackageStatus.UPDATE,
                "85MB",
                "Telegram desktop",
            ),
            Package(
                "htop",
                "3.3.0",
                "apt",
                PackageStatus.INSTALLED,
                "2MB",
                "Interactive process viewer",
            ),
        ]

        self._init = True
        self.refresh_table()

        # Select first
        table = self.query_one("#table", CleanTable)
        table.move_cursor(row=0)
        if self.packages:
            self.selected = self.packages[0]
            self.query_one("#info", InfoPanel).show(self.selected)

        self.query_one("#stats", StatsBar).update_stats(self.packages, self.tasks)

        # Start progress updater
        self.set_interval(0.1, self.update_progress)

    def refresh_table(self):
        """Refresh table with current filter"""
        if not self._init:
            return

        table = self.query_one("#table", CleanTable)
        if not table:
            return

        table.clear()

        # Filter and search
        filtered = []
        for pkg in self.packages:
            # Source filter
            if (
                self.filter != "all"
                and self.filter != "updates"
                and pkg.source != self.filter
            ):
                continue
            # Updates filter - show only packages with updates
            if self.filter == "updates" and pkg.status != PackageStatus.UPDATE:
                continue
            # Search filter
            if self.search_query:
                query = self.search_query.lower()
                if (
                    query not in pkg.name.lower()
                    and query not in pkg.description.lower()
                ):
                    continue
            filtered.append(pkg)

        for pkg in filtered:
            try:
                table.add_pkg(pkg)
            except Exception:
                pass

        # Update stats
        self.query_one("#stats", StatsBar).update_stats(self.packages, self.tasks)

    def watch_filter(self, value):
        if self._init:
            self.refresh_table()

    def watch_search_query(self, value):
        if self._init:
            self.refresh_table()

    def update_progress(self):
        """Update task progress"""
        changed = False
        for task in self.tasks:
            if task.status == "running":
                task.progress += 2
                if task.progress >= 100:
                    task.progress = 100
                    task.status = "done"
                changed = True
            elif task.status == "queued":
                task.status = "running"
                changed = True

        if changed:
            self.query_one("#queue", QueuePanel).update_tasks(self.tasks)
            self.query_one("#stats", StatsBar).update_stats(self.packages, self.tasks)

    def on_data_table_row_highlighted(self, event):
        table = self.query_one("#table", CleanTable)
        row_key = event.row_key.value

        # Find package
        for pkg in self.packages:
            if pkg.name == row_key:
                self.selected = pkg
                self.query_one("#info", InfoPanel).show(pkg)
                break

    def on_button_pressed(self, event):
        if event.button.id == "clear-btn":
            self.search_query = ""
            self.query_one("#search", Input).value = ""
            self.notify("Search cleared")

    async def on_input_changed(self, event: Input.Changed):
        if event.input.id == "search":
            await asyncio.sleep(0.1)  # Debounce
            self.search_query = event.value

    def action_search(self):
        self.query_one("#search", Input).focus()

    def action_filter_all(self):
        self.filter = "all"
        self.notify("All packages")

    def action_filter_apt(self):
        self.filter = "apt"
        self.notify("Apt packages")

    def action_filter_flatpak(self):
        self.filter = "flatpak"
        self.notify("Flatpak packages")

    def action_filter_cargo(self):
        self.filter = "cargo"
        self.notify("Cargo packages")

    def action_filter_updates(self):
        self.filter = "updates"
        self.notify("Updates available")

    def action_install(self):
        if self.selected:
            self.tasks.append(Task(self.selected.name, "install"))
            self.query_one("#queue", QueuePanel).update_tasks(self.tasks)
            self.notify(f"Installing {self.selected.name}...", severity="information")

    def action_update(self):
        if self.selected:
            self.tasks.append(Task(self.selected.name, "update"))
            self.query_one("#queue", QueuePanel).update_tasks(self.tasks)
            self.notify(f"Updating {self.selected.name}...", severity="warning")

    def action_remove(self):
        if self.selected:
            self.tasks.append(Task(self.selected.name, "remove"))
            self.query_one("#queue", QueuePanel).update_tasks(self.tasks)
            self.notify(f"Removing {self.selected.name}...", severity="error")

    def action_demo(self):
        """Add demo tasks"""
        self.tasks.append(Task("rust-analyzer", "install"))
        self.tasks.append(Task("fd-find", "install"))
        self.tasks.append(Task("exa", "install"))
        self.query_one("#queue", QueuePanel).update_tasks(self.tasks)
        self.notify("Demo tasks started!", severity="success")

    def action_clear(self):
        """Clear completed tasks"""
        self.tasks = [t for t in self.tasks if t.status != "done"]
        self.query_one("#queue", QueuePanel).update_tasks(self.tasks)
        self.notify("Cleared completed tasks")


if __name__ == "__main__":
    app = LinGetApp()
    app.run()
