# TUI Update Center E2E Checklist

## Scope
- Validate Update Center UX end-to-end in `linget tui`.
- Cover update selection paths: recommended, by-source, all, and custom selected.
- Validate retry, ignore/snooze persistence, hidden updates view, and queue summaries.

## Prerequisites
- Build passes locally: `cargo build`
- Run TUI: `cargo run -- tui`
- Ensure system has at least 3 updates across at least 2 providers (if available).

## Keyboard Quick Map
- `u`: open/close Update Center
- `U`: queue recommended updates
- `B`: queue updates for selected package source
- `S`: queue selected updates
- `A`: queue all visible updates
- `F`: retry failed update tasks
- `Space`: toggle selection
- `m`: mark recommended updates
- `i`: ignore/unignore selected update
- `z`: snooze selected update for 24h
- `Z`: clear snooze
- `v`: toggle active/hidden updates view
- `l`: toggle verbose/quiet live logs

## Scenario 1: Recommended Path
- [ ] Open Update Center (`u`).
- [ ] Confirm summary panel shows security/recommended/optional/risky counts.
- [ ] Queue recommended updates (`U`) and confirm (`y`).
- [ ] Queue panel shows progress (`done/total`), active task label, and fail counter.

## Scenario 2: By-Source Path
- [ ] Select package from provider X.
- [ ] Queue by-source updates (`B`) and confirm (`y`).
- [ ] Confirm queued tasks only contain provider X packages.

## Scenario 3: Custom Selection Path
- [ ] Select multiple updates with `Space` (or `m` then adjust manually).
- [ ] Queue selected updates (`S`) and confirm (`y`).
- [ ] Confirm queue count equals selected count shown in summary.

## Scenario 4: Update All Path
- [ ] Queue all visible updates (`A`) and confirm (`y`).
- [ ] Confirm all visible updates are queued.

## Scenario 5: Failure Recovery
- [ ] Force/observe at least one failed update task.
- [ ] Confirm failed task appears in queue with failed status.
- [ ] Use retry failed (`F`) and verify only failed update tasks are re-queued.
- [ ] Verify run finishes and summary shows success/failure/cancelled counts.

## Scenario 6: Ignore and Snooze
- [ ] Ignore an update (`i`) in active view; it should disappear.
- [ ] Snooze another update (`z`) in active view; it should disappear.
- [ ] Toggle hidden view (`v`) and confirm ignored/snoozed updates are visible.
- [ ] Clear snooze (`Z`) or unignore (`i`) and confirm item returns to active view.
- [ ] Restart app and verify ignored/snoozed state persists.

## Scenario 7: Log Ergonomics
- [ ] Toggle logs to quiet (`l`) while queue runs.
- [ ] Confirm operation continues and queue state/progress remains correct.
- [ ] Toggle logs back to verbose (`l`) and confirm new live logs are shown.

## Exit Criteria
- [ ] All scenarios pass without panics or broken layout.
- [ ] `cargo fmt --check`
- [ ] `cargo clippy -- -D warnings`
- [ ] `cargo test`
