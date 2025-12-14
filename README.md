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

## Usage

- **Navigation**: Use the sidebar to switch between "All Packages" and "Updates".
- **Providers**: Use the sidebar "Providers" switches to enable/disable package sources (unavailable providers are dimmed).
- **Filtering**: Use the top toolbar `Source: …` popover to filter the current view by a single source.
- **Selection**: Toggle "Selection Mode" (Ctrl+S) to select multiple packages for bulk actions.
- **Details**: Click on any package row to view more details and options.

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+F` | Search packages |
| `Ctrl+R` | Refresh package list |
| `Ctrl+S` | Toggle Selection Mode |
| `Ctrl+,` | Open Preferences |
| `Ctrl+Q` | Quit |

## Configuration

Configuration is stored in `~/.config/linget/config.toml`. You can edit this file manually or use the Preferences dialog in the app.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to get started.

## License

This project is licensed under the GPL-3.0 License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [GTK4](https://gtk.org/) and [Libadwaita](https://gnome.pages.gitlab.gnome.org/libadwaita/)
- Written in [Rust](https://www.rust-lang.org/)
