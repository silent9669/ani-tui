#!/usr/bin/env bash
# macOS Installation Script for ani-tui
# Usage: curl -fsSL https://raw.githubusercontent.com/silent9669/ani-tui/main/packaging/macos/install.sh | bash

set -euo pipefail

REPO="silent9669/ani-tui"
INSTALL_DIR="/usr/local/bin"
TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TEMP_DIR"' EXIT

show_banner() {
    cat <<'BANNER'
  /$$$$$$  /$$   /$$ /$$$$$$    /$$$$$$$$ /$$   /$$ /$$$$$$
 /$$__  $$| $$$ | $$|_  $$_/   |__  $$__/| $$  | $$|_  $$_/
| $$  \ $$| $$$$| $$  | $$        | $$   | $$  | $$  | $$
| $$$$$$$$| $$ $$ $$  | $$ /$$$$$$| $$   | $$  | $$  | $$
| $$__  $$| $$  $$$$  | $$|______/| $$   | $$  | $$  | $$
| $$  | $$| $$\  $$$  | $$        | $$   | $$   | $$  | $$
| $$  | $$| $$ \  $$ /$$$$$$      | $$   |  $$$$$$/ /$$$$$$
|__/  |__/|__/  \__/|______/      |__/    \______/ |______/

v3.8.3
BANNER
}

status() {
    printf '[ani-tui] %s\n' "$1"
}

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        printf 'Missing required command: %s\n' "$1" >&2
        exit 1
    fi
}

download() {
    local url="$1"
    local output="$2"
    status "Downloading $(basename "$output")"
    curl --fail --location --progress-bar --output "$output" "$url"
}

show_banner

require_command curl
require_command unzip

if ! command -v mpv >/dev/null 2>&1; then
    status "mpv is required for playback. Install it before watching videos."
    status "Recommended: brew install mpv"
fi

LATEST_RELEASE="$(curl --fail --silent --location "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')"
if [ -z "$LATEST_RELEASE" ]; then
    printf 'Could not resolve latest ani-tui release.\n' >&2
    exit 1
fi

ARCH="$(uname -m)"
case "$ARCH" in
    arm64)
        ASSET="ani-tui-aarch64-apple-darwin.zip"
        ;;
    x86_64)
        ASSET="ani-tui-x86_64-apple-darwin.zip"
        ;;
    *)
        printf 'Unsupported macOS architecture: %s\n' "$ARCH" >&2
        exit 1
        ;;
esac

ZIP_PATH="${TEMP_DIR}/${ASSET}"
download "https://github.com/${REPO}/releases/download/${LATEST_RELEASE}/${ASSET}" "$ZIP_PATH"

status "Extracting ani-tui"
unzip -q "$ZIP_PATH" -d "$TEMP_DIR"
chmod +x "${TEMP_DIR}/ani-tui"

status "Installing to ${INSTALL_DIR}"
if [ ! -d "$INSTALL_DIR" ]; then
    sudo mkdir -p "$INSTALL_DIR"
fi
sudo cp "${TEMP_DIR}/ani-tui" "${INSTALL_DIR}/ani-tui"

status "Verifying installation"
"${INSTALL_DIR}/ani-tui" --version
status "Installation complete"
