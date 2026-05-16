# ani-tui

A Netflix-inspired TUI (Terminal User Interface) for streaming anime.

![Demo](demo1.gif)

![Version](https://img.shields.io/badge/version-3.8.2-blue.svg)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)

## Features

- 🎬 Stream anime and films from multiple sources (AllAnime, KKPhim, OPhim)
- 🌐 Multiple subtitle languages supported (English, Vietnamese)
- 🔍 Search and browse anime catalog
- 📺 Continue watching from where you left off
- 🖼️ Image previews in terminal (iTerm2, Kitty, Warp, Windows Terminal)
- ⌨️ Keyboard-driven interface
- 🔄 Auto-update support

## Installation

### macOS

```bash
brew tap silent9669/tap
brew install ani-tui
```

### Windows

We provide a fully interactive installer that sets up ani-tui and all its dependencies (including `mpv` and Visual C++ Redistributable).

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://github.com/silent9669/ani-tui/releases/latest/download/install-complete.ps1 -OutFile install.ps1; .\install.ps1"
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
  - Linux: `sudo apt install chafa`

## Usage

```bash
# Start ani-tui
ani-tui

# Start with a search query
ani-tui -q "Attack on Titan"

# Update the app
ani-tui --update

# Show help
ani-tui --help
```

## Key Bindings

| Key       | Action        |
| --------- | ------------- |
| `↑/↓`     | Navigate      |
| `Enter`   | Select        |
| `Esc`     | Back          |
| `Shift+S` | Change source |
| `Shift+R` | View activity logs |
| `q`       | Quit          |

## Supported Terminals

| Terminal               | Image Support |
| ---------------------- | ------------- |
| iTerm2 (macOS)         | ✅ Full       |
| Kitty                  | ✅ Full       |
| Warp                   | ✅ Full       |
| WezTerm                | ✅ Full       |
| Windows Terminal       | ✅ Stable text image fallback |
| Terminal.app           | ❌ Text only  |

On Windows, Kitty or WezTerm are recommended for normal terminal image previews. Windows Terminal uses the stable halfblock fallback by default. You can force a renderer with `ANI_TUI_IMAGE_PROTOCOL=kitty|iterm2|sixel|halfblocks|auto`.

## Documentation

- [Changelog](docs/CHANGELOG.md)
- [Image Rendering](docs/image_rendering.md)

## License

MIT License
