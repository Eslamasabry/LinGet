#!/usr/bin/env bash
set -euo pipefail

ARCHIVE=${1:?usage: scripts/verify_release.sh ARCHIVE [terminal|gui]}
FLAVOR=${2:-terminal}
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

if [[ ! -f "$ARCHIVE" || ! -f "$ARCHIVE.sha256" ]]; then
    echo "Archive or sidecar checksum is missing: $ARCHIVE" >&2
    exit 1
fi

(cd "$(dirname "$ARCHIVE")" && sha256sum --check "$(basename "$ARCHIVE").sha256")

if tar -tzf "$ARCHIVE" | grep -Eq '(^/|(^|/)\.\.(/|$))'; then
    echo "Archive contains an unsafe path" >&2
    exit 1
fi

tar -xzf "$ARCHIVE" -C "$TMP_DIR"
ROOT=$(find "$TMP_DIR" -mindepth 1 -maxdepth 1 -type d -print -quit)
BINARY="$ROOT/linget"

for file in linget install.sh README.md LICENSE SECURITY.md SUPPORT.md PRIVACY.md; do
    [[ -e "$ROOT/$file" ]] || { echo "Missing archive entry: $file" >&2; exit 1; }
done
[[ -x "$BINARY" ]] || { echo "LinGet binary is not executable" >&2; exit 1; }

if [[ "$FLAVOR" == "gui" ]]; then
    [[ -d "$ROOT/data" ]] || { echo "GUI archive is missing data/" >&2; exit 1; }
else
    [[ ! -e "$ROOT/data" ]] || { echo "Terminal archive unexpectedly contains GUI data/" >&2; exit 1; }
fi

HOST=$(rustc -vV | sed -n 's/^host: //p')
case "$(basename "$ARCHIVE")" in
    *"-$HOST.tar.gz")
        VERSION_OUTPUT=$("$BINARY" --version)
        printf '%s\n' "$VERSION_OUTPUT"
        BINARY_VERSION=$(printf '%s\n' "$VERSION_OUTPUT" | awk '$1 == "LinGet" { print $2; exit }')
        ARCHIVE_NAME=$(basename "$ARCHIVE")
        if [[ "$FLAVOR" == "gui" ]]; then
            EXPECTED_VERSION=${ARCHIVE_NAME#linget-gui-v}
        else
            EXPECTED_VERSION=${ARCHIVE_NAME#linget-v}
        fi
        EXPECTED_VERSION=${EXPECTED_VERSION%-$HOST.tar.gz}
        if [[ "$BINARY_VERSION" != "$EXPECTED_VERSION" ]]; then
            echo "Archive version '$EXPECTED_VERSION' contains LinGet '$BINARY_VERSION'" >&2
            exit 1
        fi
        "$BINARY" --help >/dev/null
        if [[ "$FLAVOR" == "terminal" ]]; then
            if ldd "$BINARY" 2>/dev/null | grep -Eiq '(gtk|gdk|adwaita|cairo|pango)'; then
                echo "Terminal artifact unexpectedly links a GUI library" >&2
                exit 1
            fi
        fi
        ;;
    *) echo "Skipping executable smoke for non-host artifact" ;;
esac

echo "Verified $FLAVOR release archive: $ARCHIVE"
