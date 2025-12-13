# LinGet Project Context & Instructions

## Project Overview
LinGet is a modern, GUI-based package manager for Linux, built with Rust and GTK4/Libadwaita. It unifies multiple package managers (APT, DNF, Pacman, Flatpak, Snap, npm, pip, Deb, AppImage) into a single interface.

## Tech Stack
- **Language:** Rust
- **GUI Toolkit:** GTK4 + Libadwaita
- **Async Runtime:** Tokio
- **Build System:** Cargo

## Project Structure
- `src/main.rs`: Entry point, sets up logging and Tokio runtime.
- `src/app.rs`: GTK application initialization, loads CSS and icons.
- `src/ui/`: UI components.
    - `window.rs`: Main application window, sidebar, and navigation logic.
    - `package_row.rs`: Widget for individual package items in the list.
    - `package_details.rs`: Dialog for viewing/managing package details.
- `src/backend/`: Package manager implementations.
    - `mod.rs`: `PackageManager` struct that orchestrates all backends.
    - `traits.rs`: `PackageBackend` trait definition.
    - `apt.rs`, `dnf.rs`, `pacman.rs`, `flatpak.rs`, etc.: Individual implementations.
- `src/models/`: Data structures.
    - `package.rs`: `Package`, `PackageSource`, and `PackageStatus` definitions.
    - `config.rs`: Configuration management.
    - `cache.rs`: Caching logic for package lists.
- `resources/`: Static assets like `style.css`.
- `data/`: Desktop files and icons.

## Key Commands

### Development
- **Run:** `cargo run`
- **Build Release:** `cargo build --release`
- **Check:** `cargo check`
- **Format:** `cargo fmt`

### Installation (Local)
- **Install Script:** `./install.sh` (Requires `sudo` for system directories)

### Release Process
The project uses GitHub Actions for automated releases.
1.  Bump version in `Cargo.toml`.
2.  Commit changes: `git commit -m "chore: bump version to x.y.z"`
3.  Tag the commit: `git tag vx.y.z`
4.  Push tag: `git push origin vx.y.z`

The workflow will:
- Build the binary.
- Create a `.deb` package.
- Create a generic `.tar.gz` tarball.
- Publish a GitHub Release with these assets.

## Architecture Notes
- **Unified Backend:** The `PackageManager` aggregates results from all active backends using `futures::future::join_all` for concurrent execution.
- **UI/Backend Separation:** UI components (like `LinGetWindow`) do not call backend commands directly on the main thread. They use `glib::spawn_future_local` to offload work to the Tokio runtime.
- **Caching:** `PackageCache` stores the last known state of packages to ensure instant startup. Background updates refresh this cache.
- **Permissions:** `pkexec` is used by backends (like `apt`, `dnf`, `pacman`) when root privileges are required for installation/removal.

## Future Tasks
- Implement "Discover" functionality (online search) for backends.
- Add support for more package managers (e.g., Zypper).
- Improve "Ignored Updates" UI management.
