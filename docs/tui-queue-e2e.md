# TUI Queue E2E Checklist

## Scope
- Validate queue execution, progress, cancel, and retry behaviors in the TUI.

## Preconditions
- Build with TUI support (`--features tui`).
- At least one package source enabled and reachable.
- Identify two small packages for install/remove testing.

## Setup
1. Build the TUI binary:
   - `cargo build --release --features tui`
2. Run the TUI:
   - `./target/release/linget tui` or `linget tui`
3. Open help (`h`) and confirm queue-related actions/keys.

## Queue Creation And Ordering
- [ ] Refresh package list (`r`).
- [ ] Select package A and install (`i`); verify a pending queue item appears.
- [ ] Select package B and install; verify queue count increments and order is preserved.
- [ ] Attempt to add the same package again; verify UI prevents duplicates or explains behavior.

## Progress
- [ ] Start queue execution (auto-start or via the queue action shown in help).
- [ ] Active task shows a progress indicator and updates over time.
- [ ] Completed task moves to a success state with summary/log accessible (if available).

## Cancel
- [ ] While a task is running, trigger the queue cancel action (per help screen).
- [ ] Task transitions to canceled and stops updating.
- [ ] Queue remains consistent (no stuck "running" items, counts are correct).

## Retry
- [ ] Force a failure (disconnect network or use a package expected to fail).
- [ ] Failed task is marked as failed with an error message.
- [ ] Use retry; task returns to pending and executes again.
- [ ] Retry result is recorded without duplicating queue entries.

## UI/UX Checks
- [ ] Queue view clearly shows pending/running/failed/canceled states.
- [ ] Progress indicators keep updating without flicker.
- [ ] Errors are readable and provide enough detail to diagnose failures.

## Notes
- Record package names, sources, and outcomes for regressions.
