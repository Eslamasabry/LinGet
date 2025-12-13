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

## Features

- **Unified Library** - View and manage packages from multiple sources in a single list.
- **Modern UI** - Built with GTK4 and Libadwaita for a native GNOME experience.
- **Smart Filtering** - Toggle sources on/off instantly to find what you need.
- **Bulk Operations** - Select multiple packages to update or remove them all at once.
- **Update Center** - See all available updates across your system in one view.
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
| **Deb** | Local .deb files | List, Install, Remove |
| **AppImage** | Portable AppImages | List, Remove |

## Installation

### Download Binary

You can download the latest pre-compiled binary from the [Releases page](https://github.com/Eslamasabry/LinGet/releases).

1.  Download `linget-v0.1.0-linux-x86_64.tar.gz`.
2.  Extract the archive: `tar -xvf linget-v0.1.0-linux-x86_64.tar.gz`
3.  Run the binary: `./linget`
4.  (Optional) Install to `/usr/local/bin`: `sudo cp linget /usr/local/bin/`

### Prerequisites

You need the following dependencies installed on your system:

**Ubuntu/Debian:**
```bash
sudo apt install libgtk-4-dev libadwaita-1-dev build-essential pkg-config
```

**Fedora:**
```bash
sudo dnf install gtk4-devel libadwaita-devel gcc pkg-config
```

**Arch Linux:**
```bash
sudo pacman -S gtk4 libadwaita base-devel pkgconf
```

### Building from Source

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Clone the repository:**
   ```bash
   git clone https://github.com/linget/linget.git
   cd linget
   ```

3. **Build and Run:**
   ```bash
   cargo run --release
   ```

4. **Install System-wide (Optional):**
   ```bash
   sudo make install
   ```

## Usage

- **Navigation**: Use the sidebar to switch between "All Packages" and "Updates".
- **Filtering**: Click the source badges in the sidebar (e.g., "APT", "Flatpak") to filter the list.
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