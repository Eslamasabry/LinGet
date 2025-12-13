#!/bin/bash
set -e

APP_NAME="linget"
APP_ID="io.github.linget"

cd "$(dirname "$0")"

echo "=== LinGet Installer ==="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check for cargo
if ! command -v cargo &> /dev/null; then
    if [ -f "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        echo -e "${RED}Error: cargo not found. Please install Rust first.${NC}"
        exit 1
    fi
fi

# Step 1: Uninstall existing version
echo -e "${YELLOW}[1/4] Removing existing installation...${NC}"

# Remove binary
if [ -f "/usr/local/bin/$APP_NAME" ]; then
    sudo rm -f "/usr/local/bin/$APP_NAME"
    echo "  Removed /usr/local/bin/$APP_NAME"
fi

# Remove desktop file
if [ -f "/usr/share/applications/$APP_ID.desktop" ]; then
    sudo rm -f "/usr/share/applications/$APP_ID.desktop"
    echo "  Removed desktop file"
fi

# Remove icons
if [ -f "/usr/share/icons/hicolor/scalable/apps/$APP_ID.svg" ]; then
    sudo rm -f "/usr/share/icons/hicolor/scalable/apps/$APP_ID.svg"
    echo "  Removed scalable icon"
fi

if [ -f "/usr/share/icons/hicolor/symbolic/apps/$APP_ID-symbolic.svg" ]; then
    sudo rm -f "/usr/share/icons/hicolor/symbolic/apps/$APP_ID-symbolic.svg"
    echo "  Removed symbolic icon"
fi

echo -e "${GREEN}  Done${NC}"

# Step 2: Build release version
echo -e "${YELLOW}[2/4] Building release version...${NC}"
cargo build --release
echo -e "${GREEN}  Done${NC}"

# Step 3: Install
echo -e "${YELLOW}[3/4] Installing...${NC}"

# Install binary
sudo install -Dm755 target/release/$APP_NAME /usr/local/bin/$APP_NAME
echo "  Installed binary to /usr/local/bin/$APP_NAME"

# Install icons
sudo install -Dm644 data/icons/hicolor/scalable/apps/$APP_ID.svg \
    /usr/share/icons/hicolor/scalable/apps/$APP_ID.svg
echo "  Installed scalable icon"

sudo install -Dm644 data/icons/hicolor/symbolic/apps/$APP_ID-symbolic.svg \
    /usr/share/icons/hicolor/symbolic/apps/$APP_ID-symbolic.svg
echo "  Installed symbolic icon"

# Install desktop file
sudo install -Dm644 data/applications/$APP_ID.desktop \
    /usr/share/applications/$APP_ID.desktop
echo "  Installed desktop file"

echo -e "${GREEN}  Done${NC}"

# Step 4: Update caches
echo -e "${YELLOW}[4/4] Updating system caches...${NC}"
sudo gtk-update-icon-cache -f /usr/share/icons/hicolor 2>/dev/null || true
sudo update-desktop-database /usr/share/applications 2>/dev/null || true
echo -e "${GREEN}  Done${NC}"

echo ""
echo -e "${GREEN}=== Installation Complete ===${NC}"
echo ""
echo "You can now run LinGet by:"
echo "  - Typing '$APP_NAME' in the terminal"
echo "  - Searching for 'LinGet' in your application menu"
echo ""
