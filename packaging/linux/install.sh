#!/bin/bash
# Linux Installation Script for ani-tui
# Usage: curl -fsSL https://raw.githubusercontent.com/silent9669/ani-tui/main/packaging/linux/install.sh | bash

set -e

echo "🎬 Installing ani-tui..."

# Detect OS
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
else
    echo "❌ Cannot detect Linux distribution"
    exit 1
fi

# Install dependencies
echo "📦 Installing dependencies..."
case $OS in
    ubuntu|debian)
        sudo apt-get update
        sudo apt-get install -y chafa mpv curl
        ;;
    arch|manjaro)
        sudo pacman -S --noconfirm chafa mpv curl
        ;;
    fedora)
        sudo dnf install -y chafa mpv curl
        ;;
    *)
        echo "⚠️  Unsupported distribution: $OS"
        echo "   Please install chafa, mpv, and curl manually"
        ;;
esac

# Download latest release
REPO="silent9669/ani-tui"
LATEST_RELEASE=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

echo "⬇️  Downloading ani-tui ${LATEST_RELEASE}..."

# Determine architecture
ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then
    ARCH="x86_64"
else
    echo "⚠️  Architecture $ARCH may not be fully supported"
    echo "   Building from source instead..."
    
    # Check for Rust
    if ! command -v cargo &> /dev/null; then
        echo "❌ Rust is not installed. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    
    # Clone and build
    TEMP_DIR=$(mktemp -d)
    git clone "https://github.com/${REPO}.git" "$TEMP_DIR"
    cd "$TEMP_DIR"
    cargo build --release
    
    sudo cp "target/release/ani-tui" "/usr/local/bin/ani-tui"
    echo "✅ ani-tui built and installed successfully!"
    exit 0
fi

BINARY_URL="https://github.com/${REPO}/releases/download/${LATEST_RELEASE}/ani-tui-linux-${ARCH}.tar.gz"

# Create temporary directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Download and extract
curl -fsSL "$BINARY_URL" | tar -xz -C "$TEMP_DIR"

# Install
INSTALL_DIR="/usr/local/bin"
if [ ! -d "$INSTALL_DIR" ]; then
    sudo mkdir -p "$INSTALL_DIR"
fi

echo "🔧 Installing to ${INSTALL_DIR}..."
sudo cp "${TEMP_DIR}/ani-tui" "${INSTALL_DIR}/ani-tui"
sudo chmod +x "${INSTALL_DIR}/ani-tui"

# Verify installation
if command -v ani-tui &> /dev/null; then
    echo "✅ ani-tui installed successfully!"
    echo ""
    echo "🚀 Usage:"
    echo "   ani-tui              # Start the app"
    echo "   ani-tui -q 'naruto'  # Search immediately"
    echo ""
    echo "📖 Run 'ani-tui --help' for more options"
else
    echo "❌ Installation failed. Please check your PATH."
    exit 1
fi