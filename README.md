# LinGet

<p align="center">
  <img src="data/icons/hicolor/scalable/apps/io.github.linget.svg" width="128" height="128" alt="LinGet Logo">
</p>

<p align="center">
  <strong>A terminal-first, unified package manager for Linux</strong>
</p>

<p align="center">
  Plan, review, and manage Linux packages from one focused TUI and CLI.
</p>

---

## Disclaimer (Read This)

**LinGet is experimental software. It is provided “AS IS”, without warranty of any kind.**  
Use it at your own risk. LinGet can run package-management commands (some may require elevated privileges), and mistakes can potentially break your system or remove software you care about.

Also: parts of this project were **vibe-coded** (rapidly prototyped with AI assistance). That means rough edges may exist, and you should review what it does before trusting it on important machines.

## Features

- **Terminal-first** - `linget` opens the interactive TUI without GTK or a desktop session.
- **Reviewed operations** - Inspect provider, command intent, fidelity, risk, and expected changes before execution.
- **Verified results** - Stable providers return a post-operation verification receipt instead of treating process exit as proof.
- **Unified catalog** - Browse installed packages and updates from multiple sources.
- **Queue workflow** - Stage package operations, review impact, and execute them serially.
- **Scriptable CLI** - Use focused subcommands and structured output outside the TUI.
- **Optional GUI** - GTK4/Libadwaita remains available as a separately built and packaged feature.

## Provider support

| Tier | Providers | What the tier means |
| --- | --- | --- |
| **Stable** | APT, Flatpak, npm | Contract-tested commands, structured failures, labeled plan fidelity, and post-operation verification. |
| **Beta** | Other implemented providers | Available for evaluation, but not yet held to the Stable contract. Review every plan and provider response. |
| **Detection-only** | Providers reported as unavailable or unsupported | Discovery only; LinGet must not claim an unsupported mutation. |

APT provides exact simulated plans where the host supports them. Flatpak and npm plans are currently best effort and are labeled as such. Read the [provider support contract](docs/provider-support.md) before relying on a mutation.

## Installation

### Quick Install (Recommended)

The installer selects the terminal artifact for your architecture, verifies its SHA-256 digest, and installs to `~/.local/bin` for a normal user (`/usr/local/bin` for root):

```bash
curl -fsSL https://raw.githubusercontent.com/Eslamasabry/LinGet/master/install.sh | bash
```

For a custom destination, download and inspect the installer first, then run `./install.sh --prefix /your/prefix`. See [Release artifacts](docs/release-artifacts.md) for manual checksum and provenance verification.

Joining the terminal-first v0.2 evaluation? Use the pinned, consent-safe
[prerelease participant guide](docs/cohort-v0.2/participant-guide.md) instead of
the stable-channel installer.

### Manual Installation

1. Download the terminal archive, its matching `.sha256` sidecar, the consolidated checksum file, and provenance attestation from the [Releases page](https://github.com/Eslamasabry/LinGet/releases).
2. Verify the checksum: `sha256sum --check linget-vVERSION-TARGET.tar.gz.sha256`.
3. Extract the archive and run its installer against the original archive: `./linget-vVERSION-TARGET/install.sh --archive "$PWD/linget-vVERSION-TARGET.tar.gz"`.

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

3. **Build and run the terminal app:**
   ```bash
   cargo run --release
   ```

   The default build has no GTK dependency. To build the optional GUI, install GTK4/Libadwaita development packages and run:

   ```bash
   cargo run --release --features gui -- gui
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

LinGet offers three interface modes. The terminal interface is the product default:

| Mode | Command | Description |
|------|---------|-------------|
| **TUI** | `linget` or `linget tui` | Interactive terminal UI (default artifact) |
| **CLI** | `linget <command>` | Command-line interface for scripting |
| **GUI** | `linget gui` | Optional GTK4/Libadwaita build and artifact |

### Privacy-safe cohort report

Prerelease participants can inspect a local summary of LinGet version, provider
readiness, and aggregate task/verification outcomes:

```bash
linget cohort-report
linget cohort-report --format json
linget cohort-report --output cohort-report.json
```

This command transmits nothing. It deliberately excludes package names and
inventories, usernames, hostnames, filesystem paths, provider command output,
errors, operation IDs, and receipt details. Review the generated report before
sharing it with the cohort facilitator. See [the cohort report privacy
contract](docs/cohort-report.md) for the exact fields and limitations.

### TUI Mode (Default)

Launch the interactive terminal interface:

```bash
linget
linget tui
```

### Optional GUI

The standard download intentionally cannot launch a GUI. Build with the `gui` feature or download the explicitly named `linget-gui-*` artifact, then run:

```bash
linget gui
```

**TUI Controls:**

| Key | Action |
|-----|--------|
| `F1` / `F2` / `F3` | Open Today / Browse / Queue |
| `Tab` | Switch visible panels in Browse |
| `j`/`k` or `↓`/`↑` | Navigate up/down |
| `g`/`G` | Jump to first/last item |
| `PageUp`/`PageDown` | Scroll by page |
| `/` or `s` | Search packages |
| `u` | Toggle updates only |
| `r` | Refresh package list |
| `i` | Install selected package |
| `x` | Remove selected package |
| `Enter` | Review the current recommendation or package action |
| `l` | Open Queue, or return from Queue |
| `?` | Show help |
| `:` | Open the command palette |
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
