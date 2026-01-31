# ani-tui

A Netflix-inspired TUI (Terminal User Interface) for streaming anime with support for English and Vietnamese subtitles.

## Features

- **Dual-Source Search**: Search both AllAnime (English) and KKPhim (Vietnamese) simultaneously with [EN]/[VN] badges
- **AniList Metadata**: Rich anime metadata including ratings, genres, and descriptions with 7-day caching
- **Fast Image Pipeline**: Parallel image downloading (10 concurrent) with chafa rendering and dual-layer caching
- **Modern Dashboard**: "Continue Watching" section with progress tracking
- **Netflix-Style Search**: Search overlay (Shift+S) with live preview panel
- **Player Controls**: Inline control menu (Next/Prev/Episodes/Download/Favorite/Back) while video plays
- **End Screen**: Post-playback options to play next episode, replay, or return to menu
- **Cross-Platform**: Works on macOS and Windows with one-command installation

## Installation

### Quick Install

**macOS (Homebrew):**
```bash
brew tap yourusername/ani-tui
brew install ani-tui
```

**Windows (Scoop):**
```powershell
scoop bucket add ani-tui https://github.com/yourusername/ani-tui
scoop install ani-tui
```

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/yourusername/ani-tui/main/packaging/windows/install.ps1 | iex
```

### Manual Installation

**Prerequisites:**
- Rust toolchain (1.70+)
- chafa (for image rendering)
- mpv (for video playback)

**Build from Source:**
```bash
git clone https://github.com/yourusername/ani-tui
cd ani-tui
cargo build --release
```

The binary will be at `target/release/ani-tui`.

**Dependencies:**
- macOS: `brew install chafa mpv`
- Windows: `scoop install chafa mpv` or `winget install HPGDorianDavis.Chafa mpv.mpv`
- Linux: `sudo apt install chafa mpv` (Debian/Ubuntu) or `sudo pacman -S chafa mpv` (Arch)

## Usage

```bash
# Start ani-tui
ani-tui

# Start with a search query
ani-tui -q "Attack on Titan"

# Use custom config file
ani-tui --config /path/to/config.toml

# Enable debug logging
ani-tui --debug
```

## Keyboard Shortcuts

### Global
| Key | Action |
|-----|--------|
| `q` | Quit |
| `Shift+S` | Open search |
| `Shift+C` | Toggle source selection |

### Search
| Key | Action |
|-----|--------|
| `↑/↓` | Navigate results |
| `Enter` | Select anime |
| `Esc` | Cancel search |
| `Shift+C` | Toggle sources |

### Player
| Key | Action |
|-----|--------|
| `n` | Next episode |
| `p` | Previous episode |
| `e` | Choose episode |
| `f` | Add to favorites |
| `b` | Back to menu |
| `Esc` | Toggle controls |

## Configuration

Create a config file at:
- **macOS**: `~/Library/Application Support/ani-tui/config.toml`
- **Windows**: `%APPDATA%\ani-tui\config.toml`
- **Linux**: `~/.config/ani-tui/config.toml`

### Example Configuration

```toml
[player]
default = "mpv"  # or "iina" on macOS

[sources]
allanime = true      # English subtitles
vietnamese = true    # Vietnamese subtitles (KKPhim)

[ui]
image_preview = true
```

## Architecture

```
ani-tui/
├── src/
│   ├── main.rs              # Entry point
│   ├── config.rs            # Configuration management
│   ├── db.rs                # SQLite database
│   ├── image/               # Image pipeline
│   ├── metadata/            # AniList integration
│   ├── player.rs            # Media player
│   ├── providers/           # Anime sources
│   │   ├── allanime.rs      # AllAnime (English)
│   │   └── kkphim.rs        # KKPhim (Vietnamese)
│   └── ui/                  # TUI components
│       ├── app.rs           # Main app logic
│       ├── components.rs    # UI components
│       ├── modern_components.rs  # New UI components
│       └── player_controller.rs  # Player controls
└── packaging/               # Distribution files
    ├── homebrew/
    ├── scoop/
    └── windows/
```

## What's New in v0.2.0

- **Dual-Source Search**: Search both English (AllAnime) and Vietnamese (KKPhim) sources
- **AniList Integration**: Rich metadata with 7-day caching
- **Image Pipeline**: Parallel downloading with chafa rendering
- **Modern UI**: Splash screen, Netflix-style search with preview panel
- **Player Controls**: Inline menu and end screen
- **Cross-Platform**: Homebrew (macOS) and Scoop (Windows) support

## Credits

Inspired by:
- [lobster](https://github.com/justchokingaround/lobster) - Shell script for streaming
- [ani-cli](https://github.com/pystardust/ani-cli) - CLI for anime streaming
- [ratatui](https://github.com/ratatui-org/ratatui) - Rust TUI library

## License

MIT License - See LICENSE file for details

## Disclaimer

This tool is for educational purposes only. Please support the anime industry by using official streaming services when available.