# ani-tui

A Netflix-inspired TUI (Terminal User Interface) for streaming anime with support for English and Vietnamese subtitles.

![Version](https://img.shields.io/badge/version-3.6.1-blue.svg)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)

## Features

- 🎬 Stream anime with English or Vietnamese subtitles
- 🔍 Search and browse anime catalog
- 📺 Continue watching from where you left off
- 🖼️ Image previews in terminal (with supported terminals)
- ⌨️ Keyboard-driven interface
- 🌐 Multi-source support (AllAnime, KKPhim)

## Supported Terminals

For the best experience with image previews, use one of these terminals:

### macOS
- **[iTerm2](https://iterm2.com/)** (Recommended) - Full image support via iTerm2 inline images protocol
- **[Kitty](https://sw.kovidgoyal.net/kitty/)** - Full image support via Kitty graphics protocol
- **[WezTerm](https://wezfurlong.org/wezterm/)** - Good compatibility
- Terminal.app - Basic functionality (no image previews)

### Windows
- **[Windows Terminal](https://aka.ms/terminal)** (Recommended) - Best compatibility
- **[WezTerm](https://wezfurlong.org/wezterm/)** - Full image support
- **[Alacritty](https://alacritty.org/)** - Good performance
- PowerShell/Command Prompt - Basic functionality

### Linux
- **[Kitty](https://sw.kovidgoyal.net/kitty/)** (Recommended) - Full image support
- **[Alacritty](https://alacritty.org/)** - Good performance
- **[WezTerm](https://wezfurlong.org/wezterm/)** - Full image support
- GNOME Terminal/Konsole - Basic functionality

**Note:** Image previews require a terminal with Sixel or inline image support. The app will work in any terminal, but images will only display in supported terminals.

## Installation

### macOS

#### Option 1: Homebrew (Recommended)
```bash
brew tap silent9669/tap
brew install ani-tui
```

#### Option 2: Install Script
```bash
curl -fsSL https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/macos/install.sh | bash
```

#### Option 3: Manual Download
1. Go to [Releases page](https://github.com/silent9669/ani-tui/releases/latest)
2. Download `ani-tui-macos-x86_64.zip` (Intel) or `ani-tui-macos-aarch64.zip` (Apple Silicon)
3. Extract and move to `/usr/local/bin`:
```bash
unzip ani-tui-macos-*.zip
mv ani-tui /usr/local/bin/
chmod +x /usr/local/bin/ani-tui
```

### Windows

#### Option 1: Complete Auto-Installer (RECOMMENDED)
Installs ani-tui + mpv + chafa automatically:

**PowerShell (Admin):**
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/windows/install-complete.ps1 | iex"
```

Or download and run: [install-all.bat](https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/windows/install-all.bat)

#### Option 2: Scoop
```powershell
scoop bucket add ani-tui https://github.com/silent9669/ani-tui
scoop install ani-tui
```

#### Option 3: Manual Download
1. Go to [Releases page](https://github.com/silent9669/ani-tui/releases/latest)
2. Download `ani-tui-windows-x86_64.zip`
3. Extract to a folder (e.g., `C:\Program Files\ani-tui`)
4. Add that folder to your PATH

### Linux

```bash
curl -fsSL https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/linux/install.sh | bash
```

## Prerequisites

- **mpv** - Required for video playback
  - macOS: `brew install mpv`
  - Windows: Included in auto-installer, or `winget install mpv`
  - Linux: `sudo apt install mpv` or `sudo pacman -S mpv`

- **chafa** (optional) - For image previews in unsupported terminals
  - macOS: `brew install chafa`
  - Windows: Included in auto-installer
  - Linux: `sudo apt install chafa`

## Usage

```bash
# Start ani-tuiani-tui

# Start with a search query
ani-tui -q "Attack on Titan"

# Show help
ani-tui --help
```

### Keyboard Shortcuts

#### Global
- `Shift+S` - Go to Search / Source Select (from Home)
- `Shift+C` - Toggle source selection (in Search)
- `Esc` / `q` - Go back / Quit

#### Dashboard (Home)
- `↑/↓` - Navigate continue watching list
- `Enter` - Resume watching
- `Shift+D` - Remove from continue watching

#### Search
- `Type` - Search anime
- `↑/↓` - Navigate results
- `Enter` - Select anime
- `Shift+C` - Change source

#### Episode Select
- `↑/↓/←/→` - Navigate episodes
- `Enter` - Play episode
- `/` - Filter episodes
- `Esc` / `b` - Back to search

#### Player Controls
- `↑/↓/←/→` - Navigate controls
- `Enter` - Activate control
- `e` - Episode list
- `n` - Next episode
- `p` - Previous episode
- `Esc` - Back to dashboard

## Troubleshooting

### Video doesn't play
You must install **mpv** before using ani-tui:
- **Windows**: Use the auto-installer or `winget install mpv`
- **macOS**: `brew install mpv`
- **Linux**: `sudo apt install mpv`

### Images not displaying
- Use a supported terminal (see [Supported Terminals](#supported-terminals))
- Install `chafa` for fallback image rendering
- Images are cached for better performance

### App crashes on startup
- Ensure you have a working internet connection
- Check that mpv is installed and in your PATH
- Try running with `--debug` flag for more information

## Building from Source

### Requirements
- Rust 1.70+ 
- OpenSSL development libraries

### Build
```bash
git clone https://github.com/silent9669/ani-tui.git
cd ani-tui
cargo build --release
```

The binary will be at `target/release/ani-tui`.

## Development

### Project Structure
```
ani-tui/
├── src/
│   ├── main.rs           # Entry point
│   ├── ui/               # UI components and screens
│   ├── providers/        # Anime source providers
│   ├── player/           # Video player integration
│   ├── db/               # Database for watch history
│   └── image/            # Image loading and caching
├── packaging/            # Installation scripts
└── .github/workflows/    # CI/CD configuration
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history and known issues.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Ratatui](https://github.com/ratatui-org/ratatui) - Rust TUI library
- Video playback via [mpv](https://mpv.io/)
- Image rendering with [Sixel](https://github.com/libsixel/libsixel) and [iTerm2](https://iterm2.com/documentation-images.html) protocols
