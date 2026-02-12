# LinGet TUI Operator E2E Checklist

## Scope
Validate TUI command discovery and execution, preflight safety guardrails, and queue failure recovery workflows.

## Preconditions
- Start from a terminal that can run the TUI.
- Ensure at least one package is visible in the package list.
- Ensure queue panel can be expanded with `l`.

## Command Palette
- Press `Ctrl+P` and verify the command palette opens.
- Close with `Esc` and verify the palette disappears without side effects.
- Open again with `:` and verify it opens.
- Type a mixed-case query (for example: `ReFr`) and verify list filtering updates live and is case-insensitive.
- Use `Up`/`Down` to change selection and verify row highlight moves.
- Press `Enter` on an enabled command (for example: `Filter favorites`) and verify the command executes.
- Select a disabled command (for example queue actions when no queue item exists) and verify a disabled reason is shown.

## Favorites Actions
- Press `f` on the current package and verify favorite state toggles.
- Select multiple packages and press `F` to favorite in bulk; press `F` again to unfavorite in bulk.
- Switch to Favorites filter (`4`) and press `v` to toggle updates-only mode.

## Preflight Guardrails
- Trigger a low-risk action (single install) and verify preflight opens with `Safe` or `Caution` risk and source breakdown.
- Confirm with `y` and verify task is queued.
- Trigger a high-risk action (remove system package source or very large batch) and verify preflight shows `High Risk` plus risk rationale.
- Press `y` once and verify operation is not queued yet (acknowledge step only).
- Press `y` again and verify operation queues.
- Press `n` from preflight and verify action is cancelled.

## Queue Failure Recovery
- Expand queue and select a failed task.
- Verify task row includes category badge (`[PERM]`, `[NET]`, `[MISS]`, `[CONF]`, or `[UNK]`).
- Verify logs panel shows:
  - Cause category and remediation action label
  - Recovery attempts and last retry outcome
- Press `M` on a failed task and verify remediation executes for category.
- Validate at least three categories in this checklist (for example `PERM`, `NET`, `MISS`).
- Press `R` on a failed task and verify retry is queued and outcome tracking updates after completion/failure.

## Session Restore
- Set non-default state:
  - source filter
  - package filter
  - search text
  - focus pane
  - package cursor position
- Exit the TUI and restart.
- Verify state is restored (filter/source/search/focus/cursor and favorites updates-only mode).

## Regression Smoke
- Queue a package operation from normal keybindings (`i`, `x`, or `u`) and verify preflight and queue flow still works.
- Open help (`?`) and verify queue hint rows include remediation keybinding `M`.
