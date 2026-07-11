#!/usr/bin/env bash
set -euo pipefail

TRACKED_IGNORED=$(git ls-files -ci --exclude-standard)
if [[ -n "$TRACKED_IGNORED" ]]; then
    echo "Tracked files are covered by .gitignore:" >&2
    printf '%s\n' "$TRACKED_IGNORED" >&2
    exit 1
fi

GENERATED_PATTERN='(^|/)(target|node_modules|__pycache__|dist)/|\.pyc$|^session-ses_.*\.md$|^linget-v.*\.tar\.gz$'
TRACKED_GENERATED=$(git ls-files | grep -E "$GENERATED_PATTERN" || true)
if [[ -n "$TRACKED_GENERATED" ]]; then
    echo "Generated artifacts are tracked:" >&2
    printf '%s\n' "$TRACKED_GENERATED" >&2
    exit 1
fi

echo "Verified: no ignored or generated artifacts are tracked"
