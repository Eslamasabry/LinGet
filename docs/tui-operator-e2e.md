# LinGet TUI Operator E2E Checklist

## Scope
Validate command discovery and execution from the TUI command palette, plus session restore behavior.

## Preconditions
- Start from a terminal that can run the TUI.
- Ensure at least one package is visible in the package list.

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
- Queue a package operation from normal keybindings (`i`, `x`, or `u`) and verify confirmation flow still works.
- Open help (`?`) and verify command palette hints are visible.
