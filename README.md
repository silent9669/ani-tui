# ani-tui

A Netflix-inspired TUI (Terminal User Interface) for streaming anime with support for English and Vietnamese subtitles.

![Version](https://img.shields.io/badge/version-3.7.4-blue.svg)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)

## Features

- 🎬 Stream anime with English or Vietnamese subtitles
- 🔍 Search and browse anime catalog
- 📺 Continue watching from where you left off
- 🖼️ Image previews in terminal (with supported terminals)
- ⌨️ Keyboard-driven interface
- 🌐 Multi-source support (AllAnime, KKPhim)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         ani-tui                              │
├─────────────────────────────────────────────────────────────┤
│  UI Layer (ratatui)                                         │
│  ├── Screens: Splash → Source → Home → Search → Episodes   │
│  ├── Image Rendering: Kitty / iTerm2 / Sixel protocols     │
│  └── Player Controls: Overlay with progress & navigation   │
├─────────────────────────────────────────────────────────────┤
│  Provider Layer                                             │
│  ├── AllAnime (English)                                    │
│  ├── KKPhim (Vietnamese)                                   │
│  └── Provider Registry (language filtering)                │
├─────────────────────────────────────────────────────────────┤
│  Data Layer                                                 │
│  ├── SQLite (watch history, cache)                         │
│  ├── Image Cache (disk + memory)                           │
│  └── Config (TOML)                                         │
└─────────────────────────────────────────────────────────────┘
```

## Installation

### macOS

```bash
brew tap silent9669/tap
brew install ani-tui
```

### Windows

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/windows/install-complete.ps1 | iex"
```

### Linux

```bash
curl -fsSL https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/linux/install.sh | bash
```

## Prerequisites

- **mpv** - Required for video playback
  - macOS: `brew install mpv`
  - Windows: Included in auto-installer
  - Linux: `sudo apt install mpv`

- **chafa** (optional) - For image previews in unsupported terminals
  - macOS: `brew install chafa`
  - Windows: Included in auto-installer
  - Linux: `sudo apt install chafa`

## Usage

```bash
# Start ani-tui
ani-tui

# Start with a search query
ani-tui -q "Attack on Titan"

# Show help
ani-tui --help
```

## Supported Terminals

| Terminal | Image Support | Protocol |
|----------|--------------|----------|
| iTerm2 (macOS) | ✅ Full | iTerm2 |
| Kitty | ✅ Full | Kitty |
| Warp | ✅ Full | iTerm2 |
| Windows Terminal 1.22+ | ✅ Full | iTerm2 |
| WezTerm | ✅ Full | iTerm2 |
| Terminal.app | ❌ None | Text only |

## Documentation

- [Changelog](docs/CHANGELOG.md)
- [Image Rendering](docs/image_rendering.md)

## Building from Source

```bash
git clone https://github.com/silent9669/ani-tui.git
cd ani-tui
cargo build --release
```

## License

MIT License - see [LICENSE](LICENSE) file for details.
