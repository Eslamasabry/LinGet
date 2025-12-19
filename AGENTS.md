# AGENTS.md

## Build & Test Commands
```bash
cargo run                        # Run dev build
cargo build --release            # Release build (or: make release)
cargo clippy -- -D warnings      # Lint (CI enforces zero warnings)
cargo fmt --check                # Format check (CI enforced)
cargo test                       # Run all tests
cargo test <name>                # Run single test by name
RUST_LOG=linget=debug cargo run  # Run with debug logging
```

## Code Style
- **Imports**: Local (`super::`/`crate::`) first, then external crates, then `std::`
- **Errors**: Use `anyhow::Result<T>` with `.context("descriptive message")` for all fallible ops
- **Naming**: `PascalCase` types, `snake_case` functions, `SCREAMING_SNAKE_CASE` constants
- **Async**: Never block GTK thread; use `glib::spawn_future_local` to offload to Tokio
- **State**: `Rc<RefCell<T>>` for UI-local state, `Arc<Mutex<T>>` for cross-thread sharing
- **Traits**: Use `#[async_trait]` for async trait methods; all backends implement `PackageBackend`
- **No suppression**: Never use `as any`, `#[allow(unused)]`, or empty `catch {}` blocks
- **Privilege**: System ops (apt, dnf) use `pkexec` via `run_pkexec()` helper
