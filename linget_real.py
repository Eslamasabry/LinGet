#!/usr/bin/env python3
"""
LinGet - Real package manager integration
Shows actual installed packages and real updates
"""

from textual.app import App, ComposeResult
from textual.widgets import DataTable, Static, Footer, Header, Input, Button
from textual.containers import Horizontal, Vertical, Container
from textual.reactive import reactive
from textual.binding import Binding
from textual import work
import subprocess
import asyncio
from datetime import datetime
from enum import Enum
import re


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


class Task:
    def __init__(self, package, action):
        self.package = package
        self.action = action
        self.progress = 0
        self.status = "queued"


class CleanTable(DataTable):
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
            pkg.size or "-",
            f"[{color}]{icon} {label}[/]",
            key=pkg.name,
        )


class InfoPanel(Static):
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
Size: {pkg.size or "Unknown"}

[dim]{pkg.description or "No description available"}[/]

[dim]Press [bold green]i[/] to install, [bold yellow]u[/] to update, [bold red]r[/] to remove[/]"""

        self.update(content)


class QueuePanel(Static):
    def update_tasks(self, tasks):
        if not tasks:
            self.update("[dim]No active tasks[/]")
            return

        lines = []
        for task in tasks[-4:]:
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
    """Status bar with counters, filter indicator, and loading spinner"""

    def update_stats(
        self, packages, tasks, active_filter="all", loading=False, loading_msg=""
    ):
        total = len(packages)
        installed = sum(1 for p in packages if p.status == PackageStatus.INSTALLED)
        updates = sum(1 for p in packages if p.status == PackageStatus.UPDATE)
        available = sum(1 for p in packages if p.status == PackageStatus.NOT_INSTALLED)
        active = sum(1 for t in tasks if t.status == "running")

        # Build filter indicator
        filters = [
            ("1", "All", "all"),
            ("2", "Apt", "apt"),
            ("3", "Flat", "flatpak"),
            ("4", "Cargo", "cargo"),
            ("5", "npm", "npm"),
            ("6", "pip", "pip"),
            ("0", "⬆", "updates"),
        ]

        filter_indicators = []
        for key, label, filter_id in filters:
            if active_filter == filter_id:
                filter_indicators.append(f"[bold white on blue] {key}:{label} ")
            else:
                filter_indicators.append(f"[dim]{key}[/]")

        if loading:
            # Show spinner animation while loading
            spinner_frames = ["⢿", "⣻", "⣽", "⣾", "⣷", "⣯", "⣟", "⡿"]
            frame = spinner_frames[
                int(datetime.now().timestamp() * 10) % len(spinner_frames)
            ]
            stats = f"{' '.join(filter_indicators)}  |  [cyan]{frame}[/] [bold]{loading_msg}[/]"
        else:
            stats = f"{' '.join(filter_indicators)}  |  [dim]Total:[/] {total}  [green]● Installed:[/] {installed}  [yellow]⬆ Updates:[/] {updates}"
            if active > 0:
                stats += f"  [cyan]▶ Running:[/] {active}"

        self.update(stats)


class LinGetApp(App):
    """Real package manager with system integration"""

    CSS = """
    Screen {
        background: $surface;
        color: $text;
    }
    
    Header {
        background: $surface;
        color: $text;
        border-bottom: solid $primary;
        height: 1;
    }
    
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
    
    #refresh-btn {
        width: 10;
        border: none;
        background: $primary-darken-1;
        color: $text;
    }
    
    #main {
        layout: vertical;
        height: 1fr;
    }
    
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
    
    #info {
        width: 30%;
        padding: 1 2;
        background: $surface;
    }
    
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
    
    #stats {
        height: 3;
        padding: 0 1;
        background: $surface-darken-1;
        border-top: solid $surface-darken-2;
        content-align: center middle;
    }
    
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
        Binding("R", "refresh", "Refresh", show=True),
        Binding("1", "filter_all", "All", show=True),
        Binding("2", "filter_apt", "Apt", show=True),
        Binding("3", "filter_flatpak", "Flatpak", show=True),
        Binding("4", "filter_cargo", "Cargo", show=True),
        Binding("5", "filter_npm", "NPM", show=True),
        Binding("6", "filter_pip", "Pip", show=True),
        Binding("0", "filter_updates", "Updates", show=True),
        Binding("c", "clear", "Clear", show=True),
    ]

    filter = reactive("all")
    search_query = reactive("")
    packages = []
    tasks = []
    selected = None
    _init = False
    loading = reactive(False)
    loading_message = reactive("Loading packages...")

    def compose(self) -> ComposeResult:
        yield Header(show_clock=False)

        with Horizontal(id="search-row"):
            yield Input(placeholder="Search packages...", id="search")
            yield Button("Clear", id="clear-btn")
            yield Button("Refresh [R]", id="refresh-btn")

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
        self.sub_title = ""

        # Load real packages
        self._init = True
        self.loading = True
        self.loading_message = "Loading packages..."
        self.load_packages()

        # Start progress updater (for task animations and spinner)
        self.set_interval(0.1, self.update_progress)

    @work(thread=True)
    def load_packages(self):
        """Load real packages from system with progress updates"""
        self.call_from_thread(lambda: setattr(self, "loading", True))
        self.call_from_thread(
            lambda: setattr(self, "loading_message", "Loading apt...")
        )
        packages = []

        # Load Apt packages
        try:
            # Get installed apt packages
            result = subprocess.run(
                ["apt", "list", "--installed"],
                capture_output=True,
                text=True,
                timeout=30,
            )

            if result.returncode == 0:
                for line in result.stdout.split("\n"):
                    if not line or line.startswith("Listing"):
                        continue
                    # Parse: 7zip/noble,now 23.01+dfsg-11 amd64 [installed,automatic]
                    parts = line.split()
                    if len(parts) >= 2:
                        name_part = parts[0]
                        name = (
                            name_part.split("/")[0] if "/" in name_part else name_part
                        )
                        version = parts[1]
                        packages.append(
                            Package(
                                name,
                                version,
                                "apt",
                                PackageStatus.INSTALLED,
                                "-",
                                "Apt package",
                            )
                        )

            self.call_from_thread(
                lambda: setattr(self, "loading_message", "Checking apt updates...")
            )
            # Get apt upgrades
            result = subprocess.run(
                ["apt", "list", "--upgradable"],
                capture_output=True,
                text=True,
                timeout=30,
            )

            if result.returncode == 0:
                for line in result.stdout.split("\n"):
                    if not line or line.startswith("Listing"):
                        continue
                    parts = line.split()
                    if len(parts) >= 2:
                        name_part = parts[0]
                        name = (
                            name_part.split("/")[0] if "/" in name_part else name_part
                        )
                        new_version = parts[1]
                        # Check if already in list and update status
                        existing = next((p for p in packages if p.name == name), None)
                        if existing:
                            existing.status = PackageStatus.UPDATE
                            existing.version = f"{existing.version} → {new_version}"
                        else:
                            packages.append(
                                Package(
                                    name,
                                    new_version,
                                    "apt",
                                    PackageStatus.UPDATE,
                                    "-",
                                    "Apt package",
                                )
                            )
        except Exception as e:
            self.call_from_thread(
                lambda: self.notify(f"Apt error: {str(e)[:50]}", severity="error")
            )

        self.call_from_thread(
            lambda: setattr(self, "loading_message", "Loading flatpak...")
        )
        # Load Flatpak packages
        try:
            result = subprocess.run(
                ["flatpak", "list", "--app"], capture_output=True, text=True, timeout=30
            )

            if result.returncode == 0:
                for line in result.stdout.split("\n"):
                    if "\t" in line:
                        parts = line.split("\t")
                        if len(parts) >= 3:
                            name = parts[0]
                            version = parts[1]
                            packages.append(
                                Package(
                                    name,
                                    version,
                                    "flatpak",
                                    PackageStatus.INSTALLED,
                                    "-",
                                    "Flatpak app",
                                )
                            )

            self.call_from_thread(
                lambda: setattr(self, "loading_message", "Checking flatpak updates...")
            )
            # Check for flatpak updates
            result = subprocess.run(
                ["flatpak", "remote-ls", "--updates"],
                capture_output=True,
                text=True,
                timeout=30,
            )

            if result.returncode == 0:
                for line in result.stdout.split("\n"):
                    if "\t" in line:
                        parts = line.split("\t")
                        if len(parts) >= 3:
                            name = parts[0]
                            new_version = parts[1]
                            existing = next(
                                (
                                    p
                                    for p in packages
                                    if p.name == name and p.source == "flatpak"
                                ),
                                None,
                            )
                            if existing:
                                existing.status = PackageStatus.UPDATE
                                existing.version = f"{existing.version} → {new_version}"
        except Exception as e:
            self.call_from_thread(
                lambda: self.notify(f"Flatpak error: {str(e)[:50]}", severity="error")
            )

        self.call_from_thread(
            lambda: setattr(self, "loading_message", "Loading cargo...")
        )
        # Load Cargo packages
        try:
            result = subprocess.run(
                ["cargo", "install", "--list"],
                capture_output=True,
                text=True,
                timeout=30,
            )

            if result.returncode == 0:
                for line in result.stdout.split("\n"):
                    if (
                        " v" in line
                        and not line.startswith(" ")
                        and not line.startswith("    ")
                    ):
                        match = re.match(r"(\S+)\s+v([\d.]+)", line)
                        if match:
                            name = match.group(1)
                            version = match.group(2)
                            packages.append(
                                Package(
                                    name,
                                    version,
                                    "cargo",
                                    PackageStatus.INSTALLED,
                                    "-",
                                    "Cargo crate",
                                )
                            )
        except Exception as e:
            self.call_from_thread(
                lambda: self.notify(f"Cargo error: {str(e)[:50]}", severity="error")
            )

        self.call_from_thread(
            lambda: setattr(self, "loading_message", "Loading npm...")
        )
        # Load npm packages
        try:
            result = subprocess.run(
                ["npm", "list", "-g", "--depth=0", "--json"],
                capture_output=True,
                text=True,
                timeout=30,
            )

            if result.returncode == 0:
                import json

                try:
                    data = json.loads(result.stdout)
                    dependencies = data.get("dependencies", {})
                    for name, info in dependencies.items():
                        if name != "":  # Skip root
                            version = info.get("version", "?")
                            packages.append(
                                Package(
                                    name,
                                    version,
                                    "npm",
                                    PackageStatus.INSTALLED,
                                    "-",
                                    "npm package",
                                )
                            )
                except json.JSONDecodeError:
                    pass
        except Exception as e:
            self.call_from_thread(
                lambda: self.notify(f"npm error: {str(e)[:50]}", severity="error")
            )

        self.call_from_thread(
            lambda: setattr(self, "loading_message", "Loading pip...")
        )
        # Load pip packages
        try:
            result = subprocess.run(
                ["pip", "list", "--format=json"],
                capture_output=True,
                text=True,
                timeout=30,
            )

            if result.returncode == 0:
                import json

                try:
                    data = json.loads(result.stdout)
                    for pkg_info in data:
                        name = pkg_info.get("name", "?")
                        version = pkg_info.get("version", "?")
                        packages.append(
                            Package(
                                name,
                                version,
                                "pip",
                                PackageStatus.INSTALLED,
                                "-",
                                "Python package",
                            )
                        )
                except json.JSONDecodeError:
                    pass
        except Exception as e:
            self.call_from_thread(
                lambda: self.notify(f"pip error: {str(e)[:50]}", severity="error")
            )

        # Update UI
        self.packages = packages
        self.call_from_thread(self.refresh_table)
        self.call_from_thread(
            lambda: self.query_one("#stats", StatsBar).update_stats(
                self.packages, self.tasks, self.filter
            )
        )
        self.call_from_thread(
            lambda: setattr(self, "sub_title", f"{len(packages)} packages loaded")
        )
        self.call_from_thread(lambda: setattr(self, "loading", False))

    def refresh_table(self):
        if not self._init:
            return

        table = self.query_one("#table", CleanTable)
        if not table:
            return

        table.clear()

        filtered = []
        for pkg in self.packages:
            if (
                self.filter != "all"
                and self.filter != "updates"
                and pkg.source != self.filter
            ):
                continue
            if self.filter == "updates" and pkg.status != PackageStatus.UPDATE:
                continue
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

        if filtered and not self.selected:
            self.selected = filtered[0]
            self.query_one("#info", InfoPanel).show(self.selected)

        self.query_one("#stats", StatsBar).update_stats(
            self.packages,
            self.tasks,
            self.filter,
            loading=self.loading,
            loading_msg=self.loading_message,
        )

    def watch_filter(self, value):
        if self._init:
            self.refresh_table()

    def watch_search_query(self, value):
        if self._init:
            self.refresh_table()

    def watch_loading(self, value):
        """Update stats bar when loading state changes"""
        if self._init:
            self.query_one("#stats", StatsBar).update_stats(
                self.packages,
                self.tasks,
                self.filter,
                loading=value,
                loading_msg=self.loading_message,
            )

    def watch_loading_message(self, value):
        """Update stats bar when loading message changes"""
        if self._init and self.loading:
            self.query_one("#stats", StatsBar).update_stats(
                self.packages, self.tasks, self.filter, loading=True, loading_msg=value
            )

    def update_progress(self):
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

        # Always update stats bar to animate spinner if loading
        if self.loading:
            self.query_one("#stats", StatsBar).update_stats(
                self.packages,
                self.tasks,
                self.filter,
                loading=True,
                loading_msg=self.loading_message,
            )
        elif changed:
            self.query_one("#queue", QueuePanel).update_tasks(self.tasks)
            self.query_one("#stats", StatsBar).update_stats(
                self.packages, self.tasks, self.filter
            )

    def on_data_table_row_highlighted(self, event):
        table = self.query_one("#table", CleanTable)
        row_key = event.row_key.value

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
        elif event.button.id == "refresh-btn":
            self.loading = True
            self.loading_message = "Refreshing packages..."
            self.load_packages()
            self.notify("Refreshing package list...")

    async def on_input_changed(self, event: Input.Changed):
        if event.input.id == "search":
            await asyncio.sleep(0.1)
            self.search_query = event.value

    def action_search(self):
        self.query_one("#search", Input).focus()

    def action_refresh(self):
        self.loading = True
        self.loading_message = "Refreshing packages..."
        self.load_packages()
        self.notify("Refreshing package list...")

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

    def action_filter_npm(self):
        self.filter = "npm"
        self.notify("npm packages")

    def action_filter_pip(self):
        self.filter = "pip"
        self.notify("pip packages")

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

    def action_clear(self):
        self.tasks = [t for t in self.tasks if t.status != "done"]
        self.query_one("#queue", QueuePanel).update_tasks(self.tasks)
        self.notify("Cleared completed tasks")


if __name__ == "__main__":
    app = LinGetApp()
    app.run()
