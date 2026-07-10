#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: scripts/package_release.sh [options]

Create a reproducible LinGet release archive.

Options:
  --target TRIPLE          Rust target triple (defaults to the host triple)
  --flavor terminal|gui    Artifact flavor (default: terminal)
  --no-build               Package an already-built binary
  --output-dir DIR         Destination directory (default: dist)
  -h, --help               Show this help
EOF
}

TARGET=""
FLAVOR="terminal"
BUILD=1
OUTPUT_DIR="dist"

while (($# > 0)); do
    case "$1" in
        --target)
            TARGET=${2:?missing value for --target}
            shift 2
            ;;
        --flavor)
            FLAVOR=${2:?missing value for --flavor}
            shift 2
            ;;
        --no-build)
            BUILD=0
            shift
            ;;
        --output-dir)
            OUTPUT_DIR=${2:?missing value for --output-dir}
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

case "$FLAVOR" in
    terminal|gui) ;;
    *)
        echo "Unsupported flavor: $FLAVOR" >&2
        exit 2
        ;;
esac

if [[ -z "$TARGET" ]]; then
    TARGET=$(rustc -vV | sed -n 's/^host: //p')
fi

APP_NAME="linget"
VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)
SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct)}
export CARGO_INCREMENTAL=0 SOURCE_DATE_EPOCH TZ=UTC LC_ALL=C

if [[ -z "$VERSION" ]]; then
    echo "Could not read package version from Cargo.toml" >&2
    exit 1
fi

FEATURE_ARGS=()
NAME_PREFIX="$APP_NAME"
if [[ "$FLAVOR" == "gui" ]]; then
    FEATURE_ARGS=(--features gui)
    NAME_PREFIX="$APP_NAME-gui"
fi

if ((BUILD)); then
    cargo build --locked --release --target "$TARGET" "${FEATURE_ARGS[@]}"
fi

BINARY="target/$TARGET/release/$APP_NAME"
if [[ ! -x "$BINARY" ]]; then
    echo "Missing release binary: $BINARY" >&2
    exit 1
fi

HOST=$(rustc -vV | sed -n 's/^host: //p')
if [[ "$TARGET" == "$HOST" ]]; then
    BINARY_VERSION=$("$BINARY" --version 2>/dev/null | awk '$1 == "LinGet" { print $2; exit }')
    if [[ "$BINARY_VERSION" != "$VERSION" ]]; then
        echo "Refusing to package $BINARY: binary version '$BINARY_VERSION' does not match Cargo.toml '$VERSION'" >&2
        exit 1
    fi
elif ! grep -aFq "$VERSION" "$BINARY"; then
    echo "Refusing to package $BINARY: cross-target binary does not contain Cargo.toml version '$VERSION'" >&2
    exit 1
fi

ARCHIVE_STEM="$NAME_PREFIX-v$VERSION-$TARGET"
STAGE_DIR="$OUTPUT_DIR/.stage/$ARCHIVE_STEM"
ARCHIVE="$OUTPUT_DIR/$ARCHIVE_STEM.tar.gz"

rm -rf "$OUTPUT_DIR/.stage"
mkdir -p "$STAGE_DIR"
install -m 0755 "$BINARY" "$STAGE_DIR/$APP_NAME"
install -m 0755 install.sh "$STAGE_DIR/install.sh"
install -m 0644 README.md LICENSE SECURITY.md SUPPORT.md PRIVACY.md "$STAGE_DIR/"

if [[ "$FLAVOR" == "gui" ]]; then
    while IFS= read -r asset; do
        install -D -m 0644 "$asset" "$STAGE_DIR/$asset"
    done < <(git ls-files data)
fi

tar \
    --sort=name \
    --format=posix \
    --pax-option=delete=atime,delete=ctime \
    --mtime="@$SOURCE_DATE_EPOCH" \
    --owner=0 \
    --group=0 \
    --numeric-owner \
    -C "$OUTPUT_DIR/.stage" \
    -cf - "$ARCHIVE_STEM" | gzip -n -9 > "$ARCHIVE"

(cd "$OUTPUT_DIR" && sha256sum "$(basename "$ARCHIVE")" > "$(basename "$ARCHIVE").sha256")
rm -rf "$OUTPUT_DIR/.stage"

printf '%s\n' "$ARCHIVE"
