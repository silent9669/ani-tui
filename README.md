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

```powershell
# Using PowerShell
iwr -useb https://raw.githubusercontent.com/silent9669/ani-tui/main/packaging/windows/install.ps1 | iex

# Or using Scoop
scoop bucket add ani-tui https://github.com/silent9669/ani-tui
scoop install ani-tui
```

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