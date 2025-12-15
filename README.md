# LinGet

<p align="center">
  <img src="data/icons/hicolor/scalable/apps/io.github.linget.svg" width="128" height="128" alt="LinGet Logo">
</p>

<p align="center">
  <strong>A modern, unified package manager for Linux</strong>
</p>

<p align="center">
  Manage all your packages from APT, Flatpak, Snap, npm, pip, and more in one beautiful, unified interface.
</p>

---

## Disclaimer (Read This)

**LinGet is experimental software. It is provided “AS IS”, without warranty of any kind.**  
Use it at your own risk. LinGet can run package-management commands (some may require elevated privileges), and mistakes can potentially break your system or remove software you care about.

Also: parts of this project were **vibe-coded** (rapidly prototyped with AI assistance). That means rough edges may exist, and you should review what it does before trusting it on important machines.

## Features

- **Unified Library** - View and manage packages from multiple sources in a single list.
- **Modern UI** - Built with GTK4 and Libadwaita for a native GNOME experience.
- **Providers + Filtering** - Enable/disable providers from the sidebar, and filter the list by source from the top toolbar.
- **Bulk Operations** - Select multiple packages to update or remove them all at once.
- **Backup & Restore** - Export your package list to a file and restore it on another machine.
- **Update Center** - See all available updates across your system in one view.
- **Ignore Updates** - Pin specific packages to prevent them from being updated.
- **Real-time Stats** - Live counters for installed packages and available updates.
- **Package Details** - View detailed info, version history, and manage individual packages.
- **Caching** - Instant startup times thanks to local caching.

## Supported Package Managers

| Source | Description | Supported Operations |
|--------|-------------|---------------------|
| **APT** | System packages (Debian/Ubuntu) | List, Install, Remove, Update |
| **Flatpak** | Sandboxed applications | List, Install, Remove, Update |
| **Snap** | Ubuntu Snap packages | List, Install, Remove, Update |
| **npm** | Global Node.js packages | List, Install, Remove, Update |
| **pip** | User Python packages | List, Install, Remove, Update |
| **pipx** | Python app packages | List, Install, Remove, Update |
| **cargo** | Rust crates | List, Install, Remove, Update |
| **brew** | Homebrew (Linuxbrew) | List, Install, Remove, Update |
| **Conda** | Conda packages (base env) | List, Install, Remove, Update |
| **Mamba** | Mamba packages (base env) | List, Install, Remove, Update |
| **Zypper** | System packages (openSUSE) | List, Install, Remove, Update |
| **AUR** | Arch User Repository | List, Check Updates, Search (Install/Remove/Update not yet) |
| **Dart** | Dart/Flutter global tools (pub global) | List, Install, Remove, Update |
| **Deb** | Local .deb files | List, Install, Remove |
| **AppImage** | Portable AppImages | List, Remove |

## Installation

### Quick Install (Recommended)

Run this one-line command to download and install the latest version:

```bash
curl -fsSL https://raw.githubusercontent.com/Eslamasabry/LinGet/main/install.sh | bash
```

### Manual Installation

1.  Download the latest release tarball from the [Releases page](https://github.com/Eslamasabry/LinGet/releases).
2.  Extract the archive.
3.  Run the installer script inside:
    ```bash
    ./install.sh
    ```

### Dependencies

### Building from Source

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Clone the repository:**
   ```bash
   git clone https://github.com/Eslamasabry/LinGet.git
   cd LinGet
   ```

3. **Build and Run:**
   ```bash
   cargo run --release
   ```

4. **Install System-wide (Optional):**
   ```bash
   make release
   sudo make install
   ```
   Note: don’t run `make release` with `sudo` (it may not have `cargo` in `PATH`).

### Local Install (No sudo)

```bash
cargo install --path .
~/.cargo/bin/linget
```

## Usage

LinGet offers three interface modes to fit your workflow:

| Mode | Command | Description |
|------|---------|-------------|
| **GUI** | `linget` or `linget gui` | Graphical interface (GTK4/Libadwaita) |
| **TUI** | `linget tui` | Interactive terminal UI (requires `--features tui`) |
| **CLI** | `linget <command>` | Command-line interface for scripting |

### GUI Mode (Default)

Launch the graphical interface:

```bash
linget        # Default - opens GUI
linget gui    # Explicit GUI launch
```

**GUI Features:**
- **Navigation**: Use the sidebar to switch between "All Packages" and "Updates"
- **Providers**: Enable/disable package sources from the sidebar
- **Filtering**: Use the top toolbar to filter by source
- **Selection**: Toggle "Selection Mode" (Ctrl+S) for bulk actions
- **Details**: Click any package for detailed info

**Keyboard Shortcuts:**

| Shortcut | Action |
|----------|--------|
| `Ctrl+F` | Search packages |
| `Ctrl+R` | Refresh package list |
| `Ctrl+S` | Toggle Selection Mode |
| `Ctrl+,` | Open Preferences |
| `Ctrl+Q` | Quit |

### TUI Mode (Terminal UI)

Launch the interactive terminal interface:

```bash
# Build with TUI support
cargo build --release --features tui

# Run TUI
linget tui
```

**TUI Controls:**

| Key | Action |
|-----|--------|
| `Tab` | Switch between Sources/Packages panels |
| `j`/`k` or `↓`/`↑` | Navigate up/down |
| `g`/`G` | Jump to first/last item |
| `PageUp`/`PageDown` | Scroll by page |
| `/` or `s` | Search packages |
| `u` | Toggle updates only |
| `r` | Refresh package list |
| `i` | Install selected package |
| `x` | Remove selected package |
| `Enter` | Show package details |
| `h` | Show help |
| `q` or `Esc` | Quit |

### CLI Mode

Use LinGet from the command line for scripting and automation:

```bash
# List all installed packages
linget list

# List packages from a specific source
linget list --source flatpak

# List only packages with updates
linget list --updates

# Search for packages
linget search firefox
linget search react --source npm

# Install a package
linget install vim --source apt
linget install discord --source flatpak -y  # Skip confirmation

# Remove a package
linget remove vim --source apt

# Update packages
linget update vim                  # Update specific package
linget update --all                # Update all packages
linget update --all --source pip   # Update all pip packages

# Show package information
linget info com.spotify.Client --source flatpak

# Check for available updates
linget check

# Manage package sources
linget sources              # List all sources
linget sources enable snap  # Enable a source
linget sources disable snap # Disable a source

# Generate shell completions
linget completions bash > ~/.bash_completion.d/linget
linget completions zsh > ~/.zsh/completions/_linget
linget completions fish > ~/.config/fish/completions/linget.fish
```

**CLI Options:**

| Option | Description |
|--------|-------------|
| `--format human` | Human-readable output (default) |
| `--format json` | JSON output for scripting |
| `-v, --verbose` | Verbose output |
| `-q, --quiet` | Minimal output |
| `-y, --yes` | Skip confirmation prompts |
| `-s, --source` | Filter by package source |

**JSON Output Example:**

```bash
$ linget list --source flatpak --format json
{
  "count": 7,
  "packages": [
    {
      "name": "com.spotify.Client",
      "version": "1.2.74",
      "source": "flatpak",
      "status": "installed",
      "size": 14889779
    }
  ]
}
```

## Configuration

Configuration is stored in `~/.config/linget/config.toml`. You can edit this file manually or use the Preferences dialog in the app.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to get started.

## License

This project is licensed under the GPL-3.0 License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [GTK4](https://gtk.org/) and [Libadwaita](https://gnome.pages.gitlab.gnome.org/libadwaita/)
- Written in [Rust](https://www.rust-lang.org/)
