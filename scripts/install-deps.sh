#!/bin/bash
# Install dependencies for LinGet

set -e

echo "Installing LinGet dependencies..."

# Detect package manager
if command -v apt &> /dev/null; then
    echo "Detected Debian/Ubuntu system"
    sudo apt update
    sudo apt install -y \
        libgtk-4-dev \
        libadwaita-1-dev \
        build-essential \
        pkg-config \
        meson \
        ninja-build

elif command -v dnf &> /dev/null; then
    echo "Detected Fedora/RHEL system"
    sudo dnf install -y \
        gtk4-devel \
        libadwaita-devel \
        gcc \
        pkg-config \
        meson \
        ninja-build

elif command -v pacman &> /dev/null; then
    echo "Detected Arch Linux system"
    sudo pacman -S --needed \
        gtk4 \
        libadwaita \
        base-devel \
        pkgconf \
        meson \
        ninja

elif command -v zypper &> /dev/null; then
    echo "Detected openSUSE system"
    sudo zypper install -y \
        gtk4-devel \
        libadwaita-devel \
        gcc \
        pkg-config \
        meson \
        ninja

else
    echo "Unknown package manager. Please install the following packages manually:"
    echo "  - GTK 4 development libraries"
    echo "  - libadwaita development libraries"
    echo "  - Build essentials (gcc, make)"
    echo "  - pkg-config"
    exit 1
fi

# Install Rust if not present
if ! command -v rustc &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

echo ""
echo "Dependencies installed successfully!"
echo "You can now build LinGet with: make release"
echo "And install with: sudo make install"
