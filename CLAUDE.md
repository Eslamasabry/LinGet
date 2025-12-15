# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LinGet is a modern, GUI-based unified package manager for Linux, built with Rust and GTK4/Libadwaita. It aggregates multiple package managers (APT, DNF, Pacman, Flatpak, Snap, npm, pip, cargo, etc.) into a single interface.

## Build & Development Commands

```bash
# Run the app (development)
cargo run

# Build release binary
cargo build --release
# or
make release

# Lint and check
cargo check
cargo clippy -- -D warnings
# or
make check

# Format code
cargo fmt

# Run with debug logging
RUST_LOG=linget=debug cargo run
# or
make dev

# Run tests
cargo test

# Install system-wide (requires sudo, run make release first)
sudo make install

# Uninstall
sudo make uninstall
```

## Architecture

### Tech Stack
- **Language:** Rust
- **GUI:** GTK4 + Libadwaita (targeting Ubuntu 22.04 compatibility: GTK 4.6, libadwaita 1.1)
- **Async Runtime:** Tokio

### Directory Structure
- `src/main.rs` - Entry point, sets up logging and Tokio runtime
- `src/app.rs` - GTK application initialization, loads CSS and icons
- `src/ui/` - UI components
  - `window.rs` - Main window, sidebar navigation, package list views
  - `package_row.rs` - Individual package item widget
  - `package_details.rs` - Package details dialog
  - `command_center.rs` - Terminal-style command output display
  - `preferences.rs` - Settings dialog
- `src/backend/` - Package manager implementations
  - `mod.rs` - `PackageManager` struct that orchestrates all backends
  - `traits.rs` - `PackageBackend` trait definition
  - Individual backend files: `apt.rs`, `flatpak.rs`, `snap.rs`, `npm.rs`, `pip.rs`, etc.
  - `pkexec.rs` - Privilege escalation handling
- `src/models/` - Data structures
  - `package.rs` - `Package`, `PackageSource`, `PackageStatus` enums
  - `config.rs` - Configuration management
  - `cache.rs` - Package list caching for instant startup

### Key Architectural Patterns

1. **Backend Trait System**: All package managers implement `PackageBackend` trait (in `traits.rs`) with methods: `is_available()`, `list_installed()`, `check_updates()`, `install()`, `remove()`, `update()`, `search()`

2. **Concurrent Backend Execution**: `PackageManager` uses `futures::future::join_all` to query all enabled backends in parallel

3. **UI/Backend Separation**: UI components use `glib::spawn_future_local` to offload backend operations to the Tokio runtime, never blocking the main GTK thread

4. **Privilege Escalation**: `pkexec` is used by system package backends (apt, dnf, pacman, zypper) when root is required

5. **Caching**: `PackageCache` stores package state locally for instant startup; background refresh updates the cache

### Adding a New Package Backend

1. Create `src/backend/newbackend.rs` implementing `PackageBackend` trait
2. Add module to `src/backend/mod.rs`
3. Add variant to `PackageSource` enum in `src/models/package.rs`
4. Register backend in `PackageManager::new()` in `src/backend/mod.rs`
5. Update `ALL_SOURCES` array in `src/ui/window.rs`

## Release Process

1. Bump version in `Cargo.toml`
2. Commit: `git commit -m "chore: bump version to x.y.z"`
3. Tag: `git tag vx.y.z`
4. Push tag: `git push origin vx.y.z`

GitHub Actions will build and publish `.deb` and `.tar.gz` releases.

## Configuration

User config stored at `~/.config/linget/config.toml`
