# LinGet R1 Release Notes (TUI Operator Experience)

## Highlights
- Full mouse support for core TUI workflows (filters, package rows, queue interactions, overlays).
- Favorites workflow completed with quick toggles, bulk operations, and favorites-only update view.
- Preflight safety modal added for install/remove/update queueing with risk classification and source impact summary.
- High-risk operations now require an explicit two-step confirmation (`acknowledge` then `confirm`).
- Failure taxonomy added to queue entries with visible category badges:
  - `PERM` permissions/authentication
  - `NET` network/connectivity
  - `MISS` not found/package unavailable
  - `CONF` lock/dependency conflict
  - `UNK` unknown
- Guided remediation shortcuts for failed queue tasks via `M`.
- Recovery tracking added to queue details (attempt count and last retry outcome).

## Operator Keybindings
- `:` / `Ctrl+P`: Open command palette
- `f`: Toggle favorite for current package
- `F`: Bulk favorite/unfavorite selected packages
- `v`: Toggle favorites updates-only mode
- `i` / `x` / `u`: Queue install/remove/update (opens preflight)
- `y` / `n`: Confirm or cancel preflight
- `l`: Expand/collapse queue panel
- `R`: Retry selected failed task
- `M`: Apply remediation for selected failed task
- `C`: Cancel selected queued task
- `[` / `]`: Scroll queue logs

## QA Gate Summary
- `cargo fmt --check`: pass
- `cargo clippy -- -D warnings`: pass
- `cargo test`: pass
- `cargo build --release`: pass
