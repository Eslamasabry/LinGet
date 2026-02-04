#!/usr/bin/env bash
set -euo pipefail

cat <<'EOF'
TUI dev smoke checklist

Build and run:
  cargo build --release --features tui
  ./target/release/linget tui

Quick keys:
  h = help  r = refresh  / = search  i = install  x = remove  q = quit

Queue and progress:
  1) Select two small packages and queue install.
  2) Confirm queue order and pending count.
  3) Let one task run and verify progress updates.

Cancel and retry:
  4) Cancel a running task using the queue action in help.
  5) Force a failure (disable network or pick a known failing package).
  6) Retry the failed task and confirm it re-queues and runs.

Reference checklist:
  docs/tui-queue-e2e.md
EOF
