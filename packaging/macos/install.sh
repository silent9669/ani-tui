#!/bin/bash
# macOS Installation Script for ani-tui
# Usage: curl -fsSL https://raw.githubusercontent.com/silent9669/ani-tui/main/packaging/macos/install.sh | bash

set -e

echo "🎬 Installing ani-tui..."

# Check if Homebrew is installed
if ! command -v brew &> /dev/null; then
    echo "❌ Homebrew is not installed. Please install Homebrew first:"
    echo "   /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
    exit 1
fi

# Check for dependencies
echo "📦 Checking dependencies..."

if ! command -v chafa &> /dev/null; then
    echo "  Installing chafa..."
    brew install chafa
fi

if ! command -v mpv &> /dev/null; then
    echo "  Installing mpv..."
    brew install mpv
fi

# Download latest release
REPO="silent9669/ani-tui"
LATEST_RELEASE=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

echo "⬇️  Downloading ani-tui ${LATEST_RELEASE}..."

# Determine architecture
ARCH=$(uname -m)
if [ "$ARCH" = "arm64" ]; then
    BINARY_URL="https://github.com/${REPO}/releases/download/${LATEST_RELEASE}/ani-tui-darwin-arm64"
else
    BINARY_URL="https://github.com/${REPO}/releases/download/${LATEST_RELEASE}/ani-tui-darwin-x86_64"
fi

# Create temporary directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Download binary
curl -fsSL -o "${TEMP_DIR}/ani-tui" "${BINARY_URL}"
chmod +x "${TEMP_DIR}/ani-tui"

# Install to /usr/local/bin
INSTALL_DIR="/usr/local/bin"
if [ ! -d "$INSTALL_DIR" ]; then
    sudo mkdir -p "$INSTALL_DIR"
fi

echo "🔧 Installing to ${INSTALL_DIR}..."
sudo cp "${TEMP_DIR}/ani-tui" "${INSTALL_DIR}/ani-tui"

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