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
    RichLog,  # Better than Log for terminal output
    Markdown,
    Tabs,
    Tab,
    Switch,  # For toggle settings
)
from textual.containers import Horizontal, Vertical, VerticalScroll, Container
from textual.reactive import reactive
from textual.binding import Binding
from textual import work
from textual.widgets.option_list import Option

import asyncio
import re
import json
import sys

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

# --- Custom Widgets ---


class PackageTable(DataTable):
    def on_mount(self):
        self.cursor_type = "row"
        # Step 20: Add checkbox column for bulk selection
        self.add_columns("☐", "Status", "Name", "Version", "Source", "Size")
        self.zebra_stripes = True
        self.selected_rows = set()  # Track selected row keys

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

            # Use composite key to avoid collisions across package managers
            row_key = f"{pkg.source}-{pkg.name}"

            # Checkbox state
            checkbox = "☑" if row_key in self.selected_rows else "☐"

            try:
                self.add_row(
                    checkbox,  # Step 20: Checkbox column
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
            "snap": "📦 Snap",
            "aur": "🗼 AUR",
            "dnf": "🎩 DNF",
            "brew": "🍺 Brew",
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
        Binding("c", "clear_tasks", "Clear Queue", show=True),
    ]

    all_packages = []
    tasks = []
    current_source = "all"
    current_mode = "mode-all"
    search_query = ""
    _running_tasks = {}  # Maps task_id -> asyncio.subprocess.Process
    selected_packages = set()  # Step 20: Track bulk selected packages by row_key
    _last_action = None  # Step 25: Track last action for undo (package, action)
    favorites = set()  # Step 22: Track favorited packages

    def compose(self) -> ComposeResult:
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
        self.title = "LinGet - Universal Package Manager"
        self.theme = "monokai"
        self._offline_mode = False  # Step 35: Track offline state
        # Step 43: Check for macOS and Homebrew
        self._is_macos = sys.platform == "darwin"
        self._has_brew = False
        if self._is_macos:
            import shutil

            self._has_brew = shutil.which("brew") is not None
        # Step 22: Load favorites from persistence
        self.favorites = load_favorites()
        self.action_refresh_data()

        # Step 34: Set up background refresh every 10 minutes
        self.set_interval(600, self._background_refresh)

        # Step 35: Check network connectivity periodically
        self.set_interval(30, self._check_network)

    def _background_refresh(self):
        """Step 34: Refresh package list in background."""
        # Only refresh if not already refreshing and not offline
        if not self._offline_mode:
            # Don't show loading overlay for background refresh
            asyncio.ensure_future(self._silent_refresh())

    async def _silent_refresh(self):
        """Perform a silent background refresh without UI blocking."""
        try:
            await self.fetch_packages()
        except Exception:
            pass  # Silent fail - don't disturb user

    def _check_network(self):
        """Step 35: Check network connectivity."""
        import urllib.request

        try:
            # Try to connect to a reliable host
            urllib.request.urlopen("https://pypi.org", timeout=3)
            if self._offline_mode:
                self._offline_mode = False
                self.notify("Back online", severity="information")
        except:
            if not self._offline_mode:
                self._offline_mode = True
                self.notify(
                    "Offline mode - remote operations disabled", severity="warning"
                )

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
            self.query_one("#term-log", RichLog).write_line(f"[cyan]INFO:[/] {msg}")

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

        log_msg("Fetching Snap packages...")
        try:
            code, out = await run_cmd(["snap", "list"])
            if code == 0:
                for line in out.splitlines()[1:]:  # Skip header line
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
        except Exception as e:
            log_msg(f"Snap error: {e}")

        log_msg("Fetching AUR packages...")
        try:
            # Check for yay first, fallback to paru
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
                # Get all installed packages from yay/paru
                code, out = await run_cmd([aur_helper, "-Q"])
                if code == 0:
                    # Get official packages to filter out AUR-only packages
                    code_official, out_official = await run_cmd(["pacman", "-Qn"])
                    official_packages = set()
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
                            # Only include if not in official repos (AUR package)
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
        except Exception as e:
            log_msg(f"AUR error: {e}")

        log_msg("Fetching DNF packages...")
        try:
            # Check if dnf is available
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
                        # Skip header lines
                        if line.startswith("Last metadata") or line.startswith(
                            "Installed"
                        ):
                            continue
                        # Parse format: name version release arch
                        parts = line.split()
                        if len(parts) >= 2 and "." in parts[0]:
                            # Extract name from "name.arch" format
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

                # Check for updates using dnf check-update
                code, out = await run_cmd(["dnf", "check-update"])
                if code == 0 or code == 100:  # 100 means updates available
                    for line in out.splitlines():
                        # Skip empty lines and headers
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
        except Exception as e:
            log_msg(f"DNF error: {e}")

        # Step 43: Homebrew Support (macOS only)
        if self._is_macos and self._has_brew:
            log_msg("Fetching Homebrew packages...")
            try:
                # Get installed formulae with versions
                code, out = await run_cmd(["brew", "list", "--versions", "--formula"])
                if code == 0:
                    for line in out.splitlines():
                        parts = line.split()
                        if len(parts) >= 2:
                            name = parts[0]
                            # Multiple versions can be installed; show first
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

                # Get installed casks with versions
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

                # Check for outdated packages
                code, out = await run_cmd(["brew", "outdated", "--quiet"])
                if code == 0:
                    outdated_names = set(
                        line.split()[0] for line in out.splitlines() if line.strip()
                    )
                    for pkg in packages:
                        if pkg.source == "brew" and pkg.name in outdated_names:
                            pkg.status = PackageStatus.UPDATE
            except Exception as e:
                log_msg(f"Homebrew error: {e}")

        # Update application state (happens natively on the async event loop thread)
        self.all_packages = sorted(packages, key=lambda p: p.name.lower())

        self.query_one("#loading-overlay").display = False
        self.apply_filters()
        log_msg("Refresh complete.")

    async def search_new_packages(self, query: str):
        """Step 16: Search for new packages across repositories."""
        from linget.search import search_new_packages as do_search

        def log_msg(msg):
            try:
                self.query_one("#loading-msg", Label).update(msg)
            except:
                pass
            self.query_one("#term-log", Log).write_line(f"[cyan]SEARCH:[/] {msg}")

        log_msg(f"Searching for '{query}'...")
        found_packages = await do_search(query, self.all_packages, self.current_source)

        if found_packages:
            log_msg(f"Found {len(found_packages)} new packages")
            # Remove previous NOT_INSTALLED packages for this source
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
            # Add new found packages
            self.all_packages.extend(found_packages)
            self.all_packages = sorted(self.all_packages, key=lambda p: p.name.lower())
        else:
            log_msg("No new packages found")

        self.apply_filters()

    def apply_filters(self):
        filtered = self.all_packages

        # Apply mode filter
        if self.current_mode == "mode-updates":
            filtered = [p for p in filtered if p.status == PackageStatus.UPDATE]
        elif self.current_mode == "mode-search":
            filtered = [p for p in filtered if p.status == PackageStatus.NOT_INSTALLED]

        # Apply source filter
        if self.current_source != "all":
            if self.current_source == "favorites":
                # Step 22: Filter to show only favorited packages
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
            elif self.current_source == "favorites":
                self.notify("No favorite packages found", severity="information")

    def update_info_panel(self, package):
        info_panel = self.query_one("#info-panel", InfoPanel)
        info_panel.package = package
        # Step 22: Re-render to show favorite status
        if package:
            for child in list(info_panel.children):
                child.remove()
            info_panel.mount(Markdown(info_panel.render_info(self.favorites)))

    # --- Event Handlers ---

    def on_button_pressed(self, event: Button.Pressed) -> None:
        if event.button.id == "refresh-btn":
            self.action_refresh_data()

    def on_switch_changed(self, event: Switch.Changed) -> None:
        """Handle offline mode and auto-refresh toggles."""
        if event.switch.id == "offline-toggle":
            self._offline_mode = event.value
            if self._offline_mode:
                self.notify("Offline mode enabled", severity="warning")
            else:
                self.notify("Online mode restored", severity="information")
        elif event.switch.id == "auto-refresh-toggle":
            # Toggle background refresh
            if event.value:
                self.set_interval(600, self._background_refresh)
                self.notify("Auto-refresh enabled (10 min)", severity="information")
            else:
                # Remove interval (Textual doesn't have remove_interval, so we'd need to track it)
                self.notify("Auto-refresh disabled", severity="warning")

    def on_input_changed(self, event: Input.Changed) -> None:
        if event.input.id == "search":
            self.search_query = event.value
            # Step 16: Trigger repository search when in "Search for New" mode
            if self.current_mode == "mode-search" and len(self.search_query) >= 2:
                asyncio.ensure_future(self.search_new_packages(self.search_query))
            else:
                self.apply_filters()

    def on_option_list_option_selected(self, event: OptionList.OptionSelected) -> None:
        # Step 22: Add favorites to the list of valid sources
        if event.option.id in (
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
        ):
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

        # Step 25: Track last action for undo (store reverse action)
        if action == "install":
            self._last_action = (pkg, "remove")
        elif action == "remove":
            self._last_action = (pkg, "install")
        elif action == "update":
            self._last_action = (pkg, "update")  # Can't undo update easily

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
        # Step 24: Confirmation for destructive actions
        info_panel = self.query_one("#info-panel", InfoPanel)
        pkg = info_panel.package
        if not pkg:
            self.notify("No package selected!", severity="warning")
            return

        # Show confirmation notification
        self.notify(
            f"Press 'r' again to confirm removing {pkg.name}",
            severity="error",
            timeout=3.0,
        )

        # Store pending removal for confirmation
        if hasattr(self, "_pending_remove") and self._pending_remove == pkg:
            # Confirmed - proceed with removal
            self._pending_remove = None
            self._queue_task("remove")
        else:
            # First press - set pending
            self._pending_remove = pkg

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

    async def action_show_dependencies(self):
        """Step 17: Show package dependencies."""
        info_panel = self.query_one("#info-panel", InfoPanel)
        pkg = info_panel.package
        if not pkg:
            self.notify("No package selected!", severity="warning")
            return

        self.query_one("#term-log", Log).write_line(
            f"[cyan]DEPS:[/] Fetching dependencies for {pkg.name}..."
        )

        deps = []
        reverse_deps = []

        if pkg.source == "apt":
            # Get dependencies
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
            except Exception as e:
                self.notify(f"Error fetching deps: {e}", severity="error")

            # Get reverse dependencies (what depends on this package)
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
            except Exception:
                pass

        # Show in log
        log = self.query_one("#term-log", Log)
        log.write_line(f"[bold]Dependencies for {pkg.name}:[/]")
        if deps:
            for dep in deps[:10]:  # Limit to first 10
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

        self.notify(f"Dependencies shown for {pkg.name}")

    def action_toggle_select(self):
        """Step 20: Toggle selection of current package."""
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

        # Refresh to show checkbox change
        self.apply_filters()
        self.notify(f"Selected: {len(self.selected_packages)} packages")

    def action_select_all(self):
        """Step 20: Select all visible packages."""
        table = self.query_one("#package-table", PackageTable)
        # Get all currently visible filtered packages
        for pkg in self._get_filtered_packages():
            row_key = f"{pkg.source}-{pkg.name}"
            table.selected_rows.add(row_key)
            self.selected_packages.add(row_key)

        self.apply_filters()
        self.notify(f"Selected all: {len(self.selected_packages)} packages")

    def action_deselect_all(self):
        """Step 20: Clear all selections."""
        table = self.query_one("#package-table", PackageTable)
        table.selected_rows.clear()
        self.selected_packages.clear()
        self.apply_filters()
        self.notify("Cleared all selections")

    def _get_filtered_packages(self):
        """Get currently filtered package list."""
        filtered = self.all_packages

        if self.current_mode == "mode-updates":
            filtered = [p for p in filtered if p.status == PackageStatus.UPDATE]
        elif self.current_mode == "mode-search":
            filtered = [p for p in filtered if p.status == PackageStatus.NOT_INSTALLED]

        if self.current_source != "all":
            filtered = [p for p in filtered if p.source == self.current_source]

        if self.search_query:
            q = self.search_query.lower()
            filtered = [
                p for p in filtered if q in p.name.lower() or q in p.description.lower()
            ]

        return filtered

    def action_bulk_install(self):
        """Step 21: Bulk install selected packages."""
        if not self.selected_packages:
            self.notify(
                "No packages selected! Press SPACE to select.", severity="warning"
            )
            return

        # Get package objects for selected row keys
        packages_to_install = []
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

        # Queue all for installation
        for pkg in packages_to_install:
            task = Task(pkg, "install")
            self.tasks.append(task)
            self.query_one("#queue-panel", QueuePanel).add_task(task)
            asyncio.ensure_future(self.run_task(task))

        self.notify(f"Bulk installing {len(packages_to_install)} packages...")

    def action_bulk_update(self):
        """Step 21: Bulk update selected packages."""
        if not self.selected_packages:
            self.notify(
                "No packages selected! Press SPACE to select.", severity="warning"
            )
            return

        # Get package objects for selected row keys
        packages_to_update = []
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

        # Queue all for update
        for pkg in packages_to_update:
            task = Task(pkg, "update")
            self.tasks.append(task)
            self.query_one("#queue-panel", QueuePanel).add_task(task)
            asyncio.ensure_future(self.run_task(task))

        self.notify(f"Bulk updating {len(packages_to_update)} packages...")

    def action_undo(self):
        """Step 25: Undo the last action by reversing it."""
        if not self._last_action:
            self.notify("Nothing to undo", severity="warning")
            return

        pkg, reverse_action = self._last_action

        if reverse_action == "update":
            self.notify("Cannot undo updates automatically", severity="error")
            return

        # Create reverse task
        task = Task(pkg, reverse_action)
        self.tasks.append(task)

        queue_panel = self.query_one("#queue-panel", QueuePanel)
        queue_panel.add_task(task)

        term = self.query_one("#term-log", Log)
        term.write_line(f"[yellow]UNDO:[/] {reverse_action.upper()} {pkg.name}")
        self.notify(f"Undoing: {reverse_action} {pkg.name}", severity="information")

        # Start the reverse task
        asyncio.ensure_future(self.run_task(task))

        # Clear last action since we've undone it
        self._last_action = None

    def action_toggle_favorite(self):
        """Step 22: Toggle favorite status for the currently selected package."""
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

        # Persist favorites
        save_favorites(self.favorites)

        # Refresh the UI to show the star
        self.apply_filters()

    async def action_show_orphans(self):
        """Step 28: Show orphan packages that can be auto-removed."""
        self.notify("Checking for orphan packages...", severity="information")

        try:
            # Run apt autoremove --dry-run to see what would be removed
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
                orphans = []

                # Parse output for packages that would be removed
                for line in output.splitlines():
                    if "Remv " in line or "Remove " in line:
                        # Extract package name
                        parts = line.split()
                        for i, part in enumerate(parts):
                            if part in ("Remv", "Remove") and i + 1 < len(parts):
                                pkg_name = parts[i + 1].split("-")[0].split("=")[0]
                                if pkg_name and pkg_name not in orphans:
                                    orphans.append(pkg_name)

                log = self.query_one("#term-log", Log)
                if orphans:
                    log.write_line(f"[bold yellow]Orphan packages ({len(orphans)}):[/]")
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
            else:
                self.notify("Failed to check for orphans", severity="error")

        except Exception as e:
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

        # Show confirmation notification (press X twice pattern)
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

        log = self.query_one("#term-log", RichLog)
        log.write_line(f"[cyan]Cleaning {self.current_source} cache...[/]")

        # For apt, show before/after cache size
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
                    log.write_line(f"[dim]Cache size before: {before_size}[/]")
            except Exception:
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
                        for line in output.splitlines()[:20]:
                            log.write_line(f"  {line}")
                else:
                    log.write_line(f"[red]Command failed: {' '.join(cmd)}[/]")
            except Exception as e:
                log.write_line(f"[red]Error running {' '.join(cmd)}: {e}[/]")

        # For apt, show after cache size and calculate freed space
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
                    log.write_line(f"[dim]Cache size after: {after_size}[/]")
                    self.notify(
                        f"APT cache cleaned (was: {before_size}, now: {after_size})",
                        severity="information",
                        timeout=5.0,
                    )
                    return
            except Exception:
                pass

        self.notify(f"{self.current_source} cache cleaned", severity="information")

    async def action_show_versions(self):
        """Step 30: Show package version history using apt-cache policy."""
        log = self.query_one("#term-log", Log)

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
                available = []

                for line in lines:
                    line = line.strip()
                    if line.startswith("Installed:"):
                        installed = line.split(":", 1)[1].strip()
                    elif line.startswith("Candidate:"):
                        candidate = line.split(":", 1)[1].strip()
                    elif line.startswith("***"):
                        # Installed version marker
                        parts = line.split()
                        if len(parts) >= 2:
                            version = parts[1]
                            repo = " ".join(parts[3:]) if len(parts) > 3 else ""
                            available.append((version, repo, True))
                    elif line.startswith(" ") and not line.startswith("   "):
                        # Other available versions
                        parts = line.split()
                        if len(parts) >= 2:
                            version = parts[0]
                            repo = " ".join(parts[2:]) if len(parts) > 2 else ""
                            available.append((version, repo, False))

                if installed and installed != "(none)":
                    log.write_line(f"  Installed: {installed}")
                else:
                    log.write_line(f"  Installed: (none)")

                if candidate and candidate != "(none)":
                    log.write_line(f"  Candidate: {candidate}")
                else:
                    log.write_line(f"  Candidate: (none)")

                if available:
                    log.write_line(f"  Available:")
                    for version, repo, is_installed in available:
                        if is_installed:
                            log.write_line(f"    • {version} (installed)")
                        else:
                            display_repo = repo.strip("()") if repo else "unknown"
                            log.write_line(f"    • {version} ({display_repo})")
                else:
                    log.write_line(f"  No versions available in repositories")

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
            log.write_line(f"[red]Error fetching versions: {e}[/]")
            self.notify(f"Error fetching versions: {e}", severity="error")

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
        elif source == "snap":
            if action == "install":
                cmd = ["pkexec", "snap", "install", name]
            elif action == "remove":
                cmd = ["pkexec", "snap", "remove", name]
            elif action == "update":
                cmd = ["pkexec", "snap", "refresh", name]
        elif source == "aur":
            # Check for yay first, fallback to paru
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
            except:
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
            # Step 43: Homebrew commands
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
            # Step 26: Save task to history
            save_task(
                package_name=task.package.name,
                package_source=task.package.source,
                action=task.action,
                status=task.status,
                error_type=task.error_type.value,
                error_message=task.error_message,
            )
            # Clean up running task reference
            self._running_tasks.pop(task.id, None)


if __name__ == "__main__":
    app = LinGetApp()
    app.run()
