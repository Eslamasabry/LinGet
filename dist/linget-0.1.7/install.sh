#!/bin/bash
set -e

REPO="Eslamasabry/LinGet"
APP_NAME="linget"
APP_ID="io.github.linget"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== LinGet Installer ===${NC}"

# Check if running as root
is_root() {
    if [ "$EUID" -ne 0 ]; then
        return 1
    fi
    return 0
}

# Function to run with sudo if needed
run_priv() {
    if is_root; then
        "$@"
    else
        sudo "$@"
    fi
}

# Check if we are in the extracted directory (local install)
if [ -f "./$APP_NAME" ] && [ -d "./data" ]; then
    echo -e "${GREEN}Installing from local directory...${NC}"
    
    echo "Installing binary..."
    run_priv install -Dm755 "$APP_NAME" "/usr/local/bin/$APP_NAME"
    
    echo "Installing icons..."
    run_priv install -Dm644 "data/icons/hicolor/scalable/apps/$APP_ID.svg" \
        "/usr/share/icons/hicolor/scalable/apps/$APP_ID.svg"
    run_priv install -Dm644 "data/icons/hicolor/symbolic/apps/$APP_ID-symbolic.svg" \
        "/usr/share/icons/hicolor/symbolic/apps/$APP_ID-symbolic.svg"
        
    echo "Installing desktop file..."
    run_priv install -Dm644 "data/applications/$APP_ID.desktop" \
        "/usr/share/applications/$APP_ID.desktop"
        
    echo "Updating system caches..."
    run_priv gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
    run_priv update-desktop-database /usr/share/applications 2>/dev/null || true
    
    echo -e "${GREEN}Success! Run '$APP_NAME' to start.${NC}"
    exit 0
fi

# Remote install mode
echo "Fetching latest release..."

# Get latest release tag
LATEST_TAG=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
    echo -e "${RED}Error: Could not find latest release.${NC}"
    exit 1
fi

echo "Latest version: $LATEST_TAG"

# Determine arch (simple check for now)
ARCH="x86_64"
FILENAME="$APP_NAME-$LATEST_TAG-linux-$ARCH.tar.gz"
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_TAG/$FILENAME"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading $DOWNLOAD_URL..."
curl -L -o "$TMP_DIR/$FILENAME" "$DOWNLOAD_URL"

echo "Extracting..."
tar -xzf "$TMP_DIR/$FILENAME" -C "$TMP_DIR"

# Find extracted dir (tarball usually contains a subdir)
EXTRACTED_DIR=$(find "$TMP_DIR" -mindepth 1 -maxdepth 1 -type d | head -n 1)

if [ -z "$EXTRACTED_DIR" ]; then
    # Fallback if tarball didn't have a root folder
    EXTRACTED_DIR="$TMP_DIR"
fi

echo "Running installer..."
cd "$EXTRACTED_DIR"
./install.sh

echo ""