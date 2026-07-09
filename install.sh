#!/usr/bin/env bash
set -euo pipefail

REPOSITORY=${LINGET_REPOSITORY:-Eslamasabry/LinGet}
VERSION=${LINGET_VERSION:-latest}
PREFIX=${PREFIX:-}
ARCHIVE_PATH=""

usage() {
    cat <<'EOF'
Install LinGet's terminal-first release with SHA-256 verification.

Usage: install.sh [options]

Options:
  --version VERSION   Install a release version such as 0.2.0 (default: latest)
  --prefix PATH       Install below PATH (default: ~/.local, or /usr/local as root)
  --archive PATH      Verify and install a downloaded release archive
  -h, --help          Show this help

Environment: LINGET_VERSION, LINGET_REPOSITORY, PREFIX
EOF
}

while (($# > 0)); do
    case "$1" in
        --version)
            VERSION=${2:?missing value for --version}
            shift 2
            ;;
        --prefix)
            PREFIX=${2:?missing value for --prefix}
            shift 2
            ;;
        --archive)
            ARCHIVE_PATH=${2:?missing value for --archive}
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

if [[ -z "$PREFIX" ]]; then
    if [[ ${EUID:-$(id -u)} -eq 0 ]]; then
        PREFIX=/usr/local
    else
        PREFIX=${HOME:?HOME must be set}/.local
    fi
fi

case "$(uname -m)" in
    x86_64|amd64) TARGET=x86_64-unknown-linux-gnu ;;
    aarch64|arm64) TARGET=aarch64-unknown-linux-gnu ;;
    *)
        echo "LinGet release binaries do not support architecture: $(uname -m)" >&2
        exit 1
        ;;
esac

need_command() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "Required command not found: $1" >&2
        exit 1
    }
}

need_command sha256sum
need_command tar

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

if [[ -z "$ARCHIVE_PATH" ]]; then
    need_command curl
    if [[ "$VERSION" == "latest" ]]; then
        RELEASE_URL="https://api.github.com/repos/$REPOSITORY/releases/latest"
        VERSION=$(curl --fail --silent --show-error --location "$RELEASE_URL" \
            | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"v\{0,1\}\([^"]*\)".*/\1/p' \
            | head -n 1)
        if [[ -z "$VERSION" ]]; then
            echo "Could not determine the latest LinGet release" >&2
            exit 1
        fi
    else
        VERSION=${VERSION#v}
    fi

    ARCHIVE_NAME="linget-v$VERSION-$TARGET.tar.gz"
    BASE_URL="https://github.com/$REPOSITORY/releases/download/v$VERSION"
    ARCHIVE_PATH="$TMP_DIR/$ARCHIVE_NAME"
    CHECKSUMS_PATH="$TMP_DIR/linget-v$VERSION-checksums.txt"

    curl --fail --silent --show-error --location "$BASE_URL/$ARCHIVE_NAME" --output "$ARCHIVE_PATH"
    curl --fail --silent --show-error --location \
        "$BASE_URL/linget-v$VERSION-checksums.txt" --output "$CHECKSUMS_PATH"

    EXPECTED=$(awk -v name="$ARCHIVE_NAME" '$2 == name { print $1; exit }' "$CHECKSUMS_PATH")
    if [[ ! "$EXPECTED" =~ ^[0-9a-fA-F]{64}$ ]]; then
        echo "Release checksum is missing or malformed for $ARCHIVE_NAME" >&2
        exit 1
    fi
    printf '%s  %s\n' "$EXPECTED" "$ARCHIVE_PATH" | sha256sum --check --status -
    echo "Verified SHA-256: $ARCHIVE_NAME"
else
    ARCHIVE_PATH=$(realpath "$ARCHIVE_PATH")
    if [[ -f "$ARCHIVE_PATH.sha256" ]]; then
        EXPECTED=$(awk 'NR == 1 { print $1 }' "$ARCHIVE_PATH.sha256")
        printf '%s  %s\n' "$EXPECTED" "$ARCHIVE_PATH" | sha256sum --check --status -
        echo "Verified SHA-256: $(basename "$ARCHIVE_PATH")"
    else
        echo "Refusing local archive without $ARCHIVE_PATH.sha256" >&2
        exit 1
    fi
fi

if tar -tzf "$ARCHIVE_PATH" | grep -Eq '(^/|(^|/)\.\.(/|$))'; then
    echo "Archive contains an unsafe path" >&2
    exit 1
fi

tar -xzf "$ARCHIVE_PATH" -C "$TMP_DIR"
BINARY=$(find "$TMP_DIR" -mindepth 2 -maxdepth 2 -type f -name linget -print -quit)
if [[ -z "$BINARY" || ! -x "$BINARY" ]]; then
    echo "Archive does not contain an executable LinGet binary" >&2
    exit 1
fi

mkdir -p "$PREFIX/bin"
if [[ ! -w "$PREFIX/bin" ]]; then
    echo "Cannot write to $PREFIX/bin; choose a writable --prefix or run with suitable privileges" >&2
    exit 1
fi
install -m 0755 "$BINARY" "$PREFIX/bin/linget"

echo "Installed LinGet to $PREFIX/bin/linget"
case ":$PATH:" in
    *":$PREFIX/bin:"*) ;;
    *) echo "Add $PREFIX/bin to PATH to run 'linget'." ;;
esac
