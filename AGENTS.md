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

## Beads (bd) Issue Tracking
- **Common commands**:
  ```bash
  bd list
  bd create "Title" --description "Context + intended change" --acceptance "How we know it's done"
  bd show <issue-id>
  bd update <issue-id> --status in_progress
  bd update <issue-id> --status closed
  bd close <issue-id>
  bd sync
  ```
- **Repo state**: Issues are stored in `.beads/issues.jsonl` (git-tracked); local cache is `.beads/beads.db` (gitignored)
- **Hooks**: If auto-sync warnings appear, run `bd hooks install`
- **Before pushing code**: Run `bd sync` so issue state is in the remote

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
