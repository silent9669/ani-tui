# Installation Guide

## macOS (Homebrew)

```bash
brew tap yourusername/ani-tui
brew install ani-tui
```

Or install directly:

```bash
brew install --cask chafa
brew install mpv
```

## Windows (Scoop)

```powershell
scoop bucket add ani-tui https://github.com/yourusername/ani-tui
scoop install ani-tui
```

## Windows (PowerShell One-Liner)

```powershell
iwr -useb https://raw.githubusercontent.com/yourusername/ani-tui/main/packaging/windows/install.ps1 | iex
```

## Manual Installation

### Prerequisites

- Rust toolchain (1.70+)
- chafa (for image rendering)
- mpv (for video playback)

### Build from Source

```bash
git clone https://github.com/yourusername/ani-tui
cd ani-tui
cargo build --release
```

The binary will be at `target/release/ani-tui`.

### Dependencies

**macOS:**
```bash
brew install chafa mpv
```

**Windows:**
```powershell
scoop install chafa mpv
# or
winget install HPGDorianDavis.Chafa mpv.mpv
```

**Linux:**
```bash
# Ubuntu/Debian
sudo apt install chafa mpv

# Arch
sudo pacman -S chafa mpv
```