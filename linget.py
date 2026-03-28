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
    Log,
    Markdown,
    Tabs,
    Tab,
)
from textual.containers import Horizontal, Vertical, VerticalScroll, Container
from textual.reactive import reactive
from textual.binding import Binding
from textual import work
from textual.widgets.option_list import Option

import asyncio
from datetime import datetime
from enum import Enum
import re
import json

# --- Data Models ---


class PackageStatus(Enum):
    INSTALLED = "installed"
    UPDATE = "update"
    NOT_INSTALLED = "available"


class ErrorType(Enum):
    """Classify task failures for better user feedback and retry strategies."""

    NONE = "none"
    AUTH_CANCELLED = "auth_cancelled"  # User cancelled pkexec/sudo
    NETWORK = "network"  # Download failed, timeout, etc.
    NOT_FOUND = "not_found"  # Package doesn't exist in repo
    CONFLICT = "conflict"  # Dependency conflict, file conflict
    LOCKED = "locked"  # dpkg/apt lock, another process running
    DISK_FULL = "disk_full"  # No space left
    PERMISSION = "permission"  # Permission denied (non-auth)
    TIMEOUT = "timeout"  # Operation timed out
    UNKNOWN = "unknown"  # Unclassified error


class Package:
    def __init__(self, name, version, source, status, size="", desc=""):
        self.name = name
        self.version = version
        self.source = source
        self.status = status
        self.size = size
        self.description = desc


class Task:
    def __init__(self, package: Package, action: str):
        self.id = (
            f"{action}-{package.source}-{package.name}-{datetime.now().timestamp():.0f}"
        )
        self.package = package
        self.action = action
        self.progress = 0.0
        self.status = "queued"
        self.error_type = ErrorType.NONE  # Step 6: Error classification
        self.error_message = ""  # Store detailed error message


# --- Custom Widgets ---


class PackageTable(DataTable):
    def on_mount(self):
        self.cursor_type = "row"
        self.add_columns("Status", "Name", "Version", "Source", "Size")
        self.zebra_stripes = True

    def populate(self, packages):
        self.clear()
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
            }.get(pkg.source, "white")

            source_logo = {
                "apt": " APT",
                "flatpak": "󰏖 Flatpak",
                "cargo": " Cargo",
                "npm": " NPM",
                "pip": " PIP",
            }.get(pkg.source, pkg.source.upper())

            # Use composite key to avoid collisions across package managers
            row_key = f"{pkg.source}-{pkg.name}"

            try:
                self.add_row(
                    status_render,
                    f"[bold]{pkg.name}[/]",
                    pkg.version,
                    f"[bold {source_color}]{source_logo}[/]",
                    pkg.size or "-",
                    key=row_key,
                )
            except Exception as e:
                import sys

                print(f"Failed to add row for {pkg.name}: {e}", file=sys.stderr)


class InfoPanel(VerticalScroll):
    package = reactive(None)

    def render_info(self):
        if not self.package:
            return "[dim italic]Select a package to view details...[/]"

        p = self.package
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
        }.get(p.source, p.source.upper())

        return f"""
# 📦 {p.name}
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

    def watch_package(self, package):
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
    def __init__(self, task: Task, **kwargs):
        super().__init__(**kwargs)
        self.task_data = task
        self.progress_bar = ProgressBar(total=100, show_eta=False)

    def compose(self) -> ComposeResult:
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

    def update_progress(self, progress: float, status: str):
        self.progress_bar.progress = progress
        status_label = self.query_one(f"#status-{self.task_data.id}", Label)

        if status == "running":
            status_label.update("[cyan]Running...[/]")
        elif status == "done":
            status_label.update("[bold green]Complete[/]")
        elif status == "error":
            status_label.update("[bold red]Failed[/]")


class QueuePanel(VerticalScroll):
    def compose(self) -> ComposeResult:
        yield Label("No active tasks.", id="empty-queue", classes="dim")

    def add_task(self, task: Task):
        empty_label = self.query("#empty-queue")
        if empty_label:
            empty_label.remove()

        row = TaskRow(task, id=f"task-row-{task.id}")
        self.mount(row)
        self.scroll_end(animate=True)
        return row


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
        Binding("escape", "cancel_task", "Cancel", show=True),
        Binding("/", "focus_search", "Search", show=True),
        Binding("ctrl+r", "refresh_data", "Refresh", show=True),
        Binding("c", "clear_tasks", "Clear Queue", show=True),
    ]

    all_packages = []
    tasks = []
    current_source = "all"
    current_mode = "mode-all"
    search_query = ""
    _running_tasks = {}  # Maps task_id -> asyncio.subprocess.Process

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True, icon="📦")

        with Vertical(id="main-layout"):
            with Horizontal(id="content-row"):
                with Vertical(id="sidebar"):
                    yield Label("Sources", classes="sidebar-title")
                    yield OptionList(
                        Option("🌍 All Sources", id="all"),
                        Option(" APT Packages", id="apt"),
                        Option("󰏖 Flatpak Apps", id="flatpak"),
                        Option(" Cargo Crates", id="cargo"),
                        Option(" NPM Packages", id="npm"),
                        Option(" PIP Packages", id="pip"),
                        id="source-list",
                    )

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
                yield Label("⚙️ Task Queue & Output", classes="panel-title")
                with TabbedContent():
                    with TabPane("📋 Tasks"):
                        yield QueuePanel(id="queue-panel")
                    with TabPane("💻 Terminal Log"):
                        yield Log(id="term-log", highlight=True)

        with Vertical(id="loading-overlay"):
            yield LoadingIndicator()
            yield Label("Initializing...", id="loading-msg")

        yield Footer()

    def on_mount(self):
        self.title = "LinGet - Universal Package Manager"
        self.theme = "monokai"
        self.action_refresh_data()

    # --- Data Loading ---

    async def fetch_packages(self):
        """Asynchronously fetch packages without blocking event loop."""
        packages = []

        def log_msg(msg):
            # Step 10: Update loading message dynamically
            try:
                self.query_one("#loading-msg", Label).update(msg)
            except:
                pass
            self.query_one("#term-log", Log).write_line(f"[cyan]INFO:[/] {msg}")

        async def run_cmd(cmd):
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
        except Exception as e:
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
        except Exception as e:
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
        except Exception as e:
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
        except Exception as e:
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
        except Exception as e:
            log_msg(f"PIP error: {e}")

        # Update application state (happens natively on the async event loop thread)
        self.all_packages = sorted(packages, key=lambda p: p.name.lower())

        self.query_one("#loading-overlay").display = False
        self.apply_filters()
        log_msg("Refresh complete.")

    def apply_filters(self):
        filtered = self.all_packages

        # Apply mode filter
        if self.current_mode == "mode-updates":
            filtered = [p for p in filtered if p.status == PackageStatus.UPDATE]
        elif self.current_mode == "mode-search":
            filtered = [p for p in filtered if p.status == PackageStatus.NOT_INSTALLED]

        # Apply source filter
        if self.current_source != "all":
            filtered = [p for p in filtered if p.source == self.current_source]

        if self.search_query:
            q = self.search_query.lower()
            filtered = [
                p for p in filtered if q in p.name.lower() or q in p.description.lower()
            ]

        table = self.query_one("#package-table", PackageTable)
        table.populate(filtered)

        if filtered:
            table.move_cursor(row=0)
            self.update_info_panel(filtered[0])
        else:
            # Step 12: Show empty state when no packages match
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

    def update_info_panel(self, package):
        self.query_one("#info-panel", InfoPanel).package = package

    # --- Event Handlers ---

    def on_button_pressed(self, event: Button.Pressed) -> None:
        if event.button.id == "refresh-btn":
            self.action_refresh_data()

    def on_input_changed(self, event: Input.Changed) -> None:
        if event.input.id == "search":
            self.search_query = event.value
            self.apply_filters()

    def on_option_list_option_selected(self, event: OptionList.OptionSelected) -> None:
        if event.option.id in ("all", "apt", "flatpak", "cargo", "npm", "pip"):
            self.current_source = event.option.id
            self.apply_filters()

    def on_tabs_tab_activated(self, event: Tabs.TabActivated) -> None:
        if event.tab.id in ("mode-all", "mode-updates", "mode-search"):
            self.current_mode = event.tab.id
            self.apply_filters()

    def on_data_table_row_highlighted(self, event: DataTable.RowHighlighted) -> None:
        row_key = event.row_key.value
        pkg = next(
            (p for p in self.all_packages if f"{p.source}-{p.name}" == row_key), None
        )
        if pkg:
            self.update_info_panel(pkg)

    # --- Actions ---

    def action_focus_search(self):
        search = self.query_one("#search", Input)
        search.focus()
        search.cursor_position = len(search.value)

    def action_refresh_data(self):
        self.query_one("#loading-overlay").display = True
        asyncio.ensure_future(self.fetch_packages())

    def _queue_task(self, action: str):
        info_panel = self.query_one("#info-panel", InfoPanel)
        pkg = info_panel.package
        if not pkg:
            self.notify("No package selected!", severity="warning")
            return

        task = Task(pkg, action)
        self.tasks.append(task)

        queue_panel = self.query_one("#queue-panel", QueuePanel)
        queue_panel.add_task(task)

        term = self.query_one("#term-log", Log)
        term.write_line(f"[yellow]QUEUED:[/] {action.upper()} {pkg.name}")
        self.notify(f"Queued: {action} {pkg.name}", severity="information")

        # Start executing the task immediately in the background
        asyncio.ensure_future(self.run_task(task))

    def action_install(self):
        self._queue_task("install")

    def action_update(self):
        self._queue_task("update")

    def action_remove(self):
        self._queue_task("remove")

    def action_clear_tasks(self):
        to_remove = [t for t in self.tasks if t.status in ("done", "error")]
        for t in to_remove:
            try:
                row = self.query_one(f"#task-row-{t.id}")
                row.remove()
            except Exception:
                pass

        self.tasks = [t for t in self.tasks if t.status not in ("done", "error")]
        if not self.tasks:
            queue_panel = self.query_one("#queue-panel", QueuePanel)
            existing = queue_panel.query("#empty-queue")
            if existing:
                existing.remove()
            queue_panel.mount(
                Label("No active tasks.", id="empty-queue", classes="dim")
            )

        self.notify("Cleared completed tasks")

    def action_cancel_task(self):
        """Cancel the currently running task (Step 3)."""
        running_task = None
        for task in self.tasks:
            if task.status == "running":
                running_task = task
                break

        if not running_task:
            self.notify("No running task to cancel", severity="warning")
            return

        # Get the process and terminate it
        process = self._running_tasks.get(running_task.id)
        if process:
            try:
                process.terminate()
                running_task.status = "cancelled"
                running_task.error_type = ErrorType.AUTH_CANCELLED
                self.notify(f"Cancelled: {running_task.package.name}")
                self.query_one("#term-log", Log).write_line(
                    f"[yellow]CANCELLED:[/] {running_task.action.upper()} {running_task.package.name}"
                )
            except Exception as e:
                self.notify(f"Failed to cancel: {e}", severity="error")

    def action_retry_task(self):
        """Retry the last failed task (Step 5)."""
        # Find most recent failed task
        failed_tasks = [t for t in self.tasks if t.status == "error"]
        if not failed_tasks:
            self.notify("No failed tasks to retry", severity="warning")
            return

        # Get most recent failed task
        task_to_retry = failed_tasks[-1]

        # Create new task for same package and action
        new_task = Task(task_to_retry.package, task_to_retry.action)
        self.tasks.append(new_task)

        queue_panel = self.query_one("#queue-panel", QueuePanel)
        queue_panel.add_task(new_task)

        self.query_one("#term-log", Log).write_line(
            f"[yellow]RETRY:[/] {new_task.action.upper()} {new_task.package.name}"
        )
        self.notify(
            f"Retrying: {new_task.action} {new_task.package.name}",
            severity="information",
        )

        # Start the task
        asyncio.ensure_future(self.run_task(new_task))

    # --- Real Task Execution ---

    async def run_task(self, task: Task):
        """Execute real package manager commands natively in the async event loop."""

        # Store process reference for cancellation
        self._running_tasks[task.id] = None

        def log_msg(msg):
            self.query_one("#term-log", Log).write_line(
                f"[{task.package.source}] {msg}"
            )

        def update_status(status, progress=None):
            task.status = status
            if progress is not None:
                task.progress = min(progress, 100.0)
            try:
                row = self.query_one(f"#task-row-{task.id}", TaskRow)
                row.update_progress(task.progress, task.status)
            except Exception as e:
                log_msg(f"[dim]UI update failed: {e}[/]")

        update_status("running", 5.0)
        self.query_one("#term-log", Log).write_line(
            f"[green]STARTED:[/] {task.action.upper()} {task.package.name}"
        )

        cmd = []
        source = task.package.source
        name = task.package.name
        action = task.action

        # Map actions to actual CLI commands
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
                # Step 7: Fix flatpak update - individual app update uses different syntax
                cmd = ["flatpak", "update", "-y", name]
        elif source == "cargo":
            if action in ("install", "update"):
                cmd = ["cargo", "install", name]
            elif action == "remove":
                # Step 9: Cargo may need privileges for system-wide uninstall
                cmd = ["cargo", "uninstall", name]
                # Note: If cargo is installed system-wide, this may fail with
                # permission denied. The error classification will catch this.
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

        if not cmd:
            log_msg(f"[red]Error:[/] Unsupported action/source combination.")
            update_status("error")
            return

        try:
            # Step 8: Show auth feedback for pkexec operations
            if source == "apt" and "pkexec" in cmd:
                update_status("running", 10.0)
                log_msg("[dim]Waiting for authentication...[/]")

            # Step 15: Add timeout for network operations
            process = await asyncio.wait_for(
                asyncio.create_subprocess_exec(
                    *cmd,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.STDOUT,
                ),
                timeout=300,  # 5 minute timeout
            )

            # Store process reference for cancellation
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
                self.query_one("#term-log", Log).write_line(
                    f"[bold green]COMPLETED:[/] {task.action.upper()} {task.package.name}"
                )
                # Step 1: Auto-refresh package list after successful operation
                self.action_refresh_data()
            else:
                # Step 6: Classify errors based on return code and output
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
            # Step 15: Handle timeout specifically
            task.error_type = ErrorType.TIMEOUT
            task.error_message = "Operation timed out after 5 minutes"
            update_status("error")
            log_msg(f"[red]Timeout:[/] {task.error_message}")
            self.notify(f"Timeout: {action} {name}", severity="error")
            # Kill the process if it's still running
            if task.id in self._running_tasks:
                try:
                    self._running_tasks[task.id].kill()
                except:
                    pass

        except Exception as e:
            error_str = str(e).lower()
            error_msg = str(e)

            # Step 6: Error classification
            if "cancel" in error_str or "terminate" in error_str:
                task.error_type = ErrorType.AUTH_CANCELLED
                task.error_message = "Operation cancelled by user"
            elif "lock" in error_str or "dpkg" in error_str and "lock" in error_str:
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
            # Clean up running task reference
            self._running_tasks.pop(task.id, None)


if __name__ == "__main__":
    app = LinGetApp()
    app.run()
