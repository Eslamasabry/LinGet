#!/usr/bin/env bash
set -euo pipefail

BINARY=${1:-target/release/linget}
REPORT=$(mktemp)
trap 'rm -f "$REPORT"' EXIT

"$BINARY" cohort-report --format json > "$REPORT"

python3 - "$REPORT" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    report = json.load(handle)

assert report["schema_version"] == 1
assert report["linget_version"]
assert report["data_handling"] == {
    "local_only": True,
    "transmitted_by_linget": False,
    "package_inventory_included": False,
    "personal_identifiers_included": False,
    "arbitrary_command_output_included": False,
}
assert isinstance(report["providers"], list)
assert isinstance(report["task_outcomes"], dict)
assert isinstance(report["verification_outcomes"], dict)
assert isinstance(report["next_steps"], list)
PY

echo "Verified privacy-safe cohort report JSON: $BINARY"
