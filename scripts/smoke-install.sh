#!/usr/bin/env bash
set -euo pipefail

SMOKE_DIR="$(mktemp -d)"
SMOKE_PREFIX="$SMOKE_DIR/prefix"
BINARY="$SMOKE_PREFIX/bin/linget"
DESKTOP_FILE="$SMOKE_PREFIX/share/applications/io.github.linget.desktop"
GUI_LOG="$SMOKE_DIR/gui.log"

cleanup() {
    rm -rf "$SMOKE_DIR"
}
trap cleanup EXIT

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Missing required command: $1" >&2
        exit 1
    fi
}

require_command make
require_command timeout

if [[ ! -x target/release/linget ]]; then
    echo "Missing target/release/linget. Run 'cargo build --release' first." >&2
    exit 1
fi

mkdir -p "$SMOKE_PREFIX" "$SMOKE_DIR/home" "$SMOKE_DIR/config" "$SMOKE_DIR/data"

make install PREFIX="$SMOKE_PREFIX"

test -x "$BINARY"

EXEC_LINE="$(grep '^Exec=' "$DESKTOP_FILE")"
EXPECTED_EXEC="Exec=$SMOKE_PREFIX/bin/linget"
if [[ "$EXEC_LINE" != "$EXPECTED_EXEC" ]]; then
    echo "Desktop Exec mismatch: got '$EXEC_LINE', expected '$EXPECTED_EXEC'" >&2
    exit 1
fi

"$BINARY" --version

dbus_prefix=()
if command -v dbus-run-session >/dev/null 2>&1; then
    dbus_prefix=(dbus-run-session --)
fi

display_prefix=()
launch_mode=""
launch_env=(
    "HOME=$SMOKE_DIR/home"
    "XDG_CONFIG_HOME=$SMOKE_DIR/config"
    "XDG_DATA_HOME=$SMOKE_DIR/data"
    "NO_AT_BRIDGE=1"
    "RUST_LOG=linget=info"
)

if command -v xvfb-run >/dev/null 2>&1; then
    display_prefix=(xvfb-run --auto-servernum --server-args="-screen 0 1024x768x24")
    launch_env+=("GDK_BACKEND=x11")
    launch_mode="xvfb-run"
elif [[ -n "${DISPLAY:-}" ]]; then
    launch_mode="existing DISPLAY"
elif [[ -n "${WAYLAND_DISPLAY:-}" ]]; then
    launch_mode="existing WAYLAND_DISPLAY"
else
    echo "Missing xvfb-run and no DISPLAY/WAYLAND_DISPLAY available for GUI smoke test." >&2
    exit 1
fi

echo "Running GUI smoke with $launch_mode"

launch_exit=0
env "${launch_env[@]}" \
    "${display_prefix[@]}" \
    "${dbus_prefix[@]}" \
    timeout 10s "$BINARY" >"$GUI_LOG" 2>&1 || launch_exit=$?

cat "$GUI_LOG"

if [[ "$launch_exit" -ne 0 && "$launch_exit" -ne 124 ]]; then
    echo "GUI smoke test failed with exit code $launch_exit" >&2
    exit 1
fi

grep -q "Starting LinGet" "$GUI_LOG"
