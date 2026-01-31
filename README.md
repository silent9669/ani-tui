# ani-tui

A Netflix-inspired TUI (Terminal User Interface) for streaming anime with support for English and Vietnamese subtitles.

## Installation

### macOS

```bash
# Using install script
curl -fsSL https://raw.githubusercontent.com/silent9669/ani-tui/main/packaging/macos/install.sh | bash

# Or using Homebrew
brew tap silent9669/tap
brew install ani-tui
```

### Windows

**Option 1: Easy Install (One Command)**
```powershell
powershell -Command "iwr -useb https://raw.githubusercontent.com/silent9669/ani-tui/main/packaging/windows/install-easy.ps1 | iex"
```

**Option 2: Download and Double-Click**
1. Download [install.bat](https://raw.githubusercontent.com/silent9669/ani-tui/main/packaging/windows/install.bat)
2. Double-click the file to install

**Option 3: Scoop**
```powershell
scoop bucket add ani-tui https://github.com/silent9669/ani-tui
scoop install ani-tui
```

[Full Windows Installation Guide](packaging/windows/README.md)

### Linux

```bash
curl -fsSL https://raw.githubusercontent.com/silent9669/ani-tui/main/packaging/linux/install.sh | bash
```

## Usage

```bash
# Start ani-tui
ani-tui

# Start with a search query
ani-tui -q "Attack on Titan"
```

## License

MIT License - See LICENSE file for details