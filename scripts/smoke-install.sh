#!/usr/bin/env bash
set -euo pipefail

SMOKE_DIR=$(mktemp -d)
SMOKE_PREFIX="$SMOKE_DIR/prefix"
SOURCE_BINARY=target/release/linget
INSTALLED_BINARY="$SMOKE_PREFIX/bin/linget"

cleanup() {
    rm -rf "$SMOKE_DIR"
}
trap cleanup EXIT

if [[ ! -x "$SOURCE_BINARY" ]]; then
    echo "Missing $SOURCE_BINARY. Run 'cargo build --release' first." >&2
    exit 1
fi

install -Dm755 "$SOURCE_BINARY" "$INSTALLED_BINARY"
"$INSTALLED_BINARY" --version
"$INSTALLED_BINARY" --help >/dev/null

gui_log="$SMOKE_DIR/gui.log"
gui_exit=0
"$INSTALLED_BINARY" gui >"$gui_log" 2>&1 || gui_exit=$?
if [[ "$gui_exit" -ne 2 ]]; then
    cat "$gui_log" >&2
    echo "Terminal artifact GUI command exited $gui_exit instead of 2" >&2
    exit 1
fi
grep -q "GUI support is not included in this build" "$gui_log"

if ldd "$INSTALLED_BINARY" 2>/dev/null | grep -Eiq '(gtk|gdk|adwaita|cairo|pango)'; then
    echo "Terminal artifact unexpectedly links a GUI library" >&2
    exit 1
fi

echo "Verified installed terminal-first binary without GUI linkage"
