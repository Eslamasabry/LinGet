#!/bin/bash
set -e

APP_NAME="linget"
VERSION=$(grep '^version =' Cargo.toml | cut -d '"' -f 2)
TARGET_DIR="target/release"
DIST_DIR="dist/$APP_NAME-$VERSION"
ARCHIVE_NAME="$APP_NAME-v$VERSION-linux-x86_64.tar.gz"

echo "Packaging $APP_NAME v$VERSION..."

# Build release
cargo build --release

# Create dist structure
rm -rf dist
mkdir -p "$DIST_DIR"

# Copy binary
cp "$TARGET_DIR/$APP_NAME" "$DIST_DIR/"

# Copy assets
cp -r data "$DIST_DIR/"
cp install.sh "$DIST_DIR/"
cp README.md "$DIST_DIR/"
cp LICENSE "$DIST_DIR/"

# Create archive
cd dist
tar -czvf "$ARCHIVE_NAME" "$APP_NAME-$VERSION"
echo "Created $ARCHIVE_NAME"
