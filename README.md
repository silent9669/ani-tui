# ani-tui

A Netflix-inspired TUI (Terminal User Interface) for streaming anime with support for English and Vietnamese subtitles.

## Installation

### macOS

```bash
# Option 1: Install script (recommended)
curl -fsSL https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/macos/install.sh | bash

# Option 2: Homebrew
brew tap silent9669/tap
brew install ani-tui
```

### Windows

**🌟 Option 1: Complete Auto-Installer (RECOMMENDED)**

Installs ani-tui + mpv + chafa automatically:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/windows/install-complete.ps1 | iex"
```

Or download and double-click: [install-all.bat](https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/windows/install-all.bat)

**Option 2: Manual Download**

1. Go to [Releases page](https://github.com/silent9669/ani-tui/releases/latest)
2. Download `ani-tui-windows-x86_64.zip`
3. Extract to a folder (e.g., `C:\Program Files\ani-tui`)
4. Add that folder to your PATH

**Option 3: Scoop**

```powershell
scoop bucket add ani-tui https://github.com/silent9669/ani-tui
scoop install ani-tui
scoop install mpv
```

### Linux

```bash
curl -fsSL https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/linux/install.sh | bash
```

## Usage

```bash
# Start ani-tui
ani-tui

# Start with a search query
ani-tui -q "Attack on Titan"
```

## Prerequisites

- **mpv** - Required for video playback
  - macOS: `brew install mpv`
  - Windows: Download from [mpv.io](https://mpv.io/installation/)
  - Linux: `sudo apt install mpv`

- **chafa** (optional) - For image previews
  - macOS: `brew install chafa`
  - Windows: Download from [hpjansson.org/chafa](https://hpjansson.org/chafa/)

## Windows Requirements

### Visual C++ Redistributable (CRITICAL)

**Windows users MUST install Visual C++ Redistributable or ani-tui will not run!**

If you type `ani-tui` and nothing happens:

```powershell
winget install Microsoft.VCRedist.2015+.x64
```

Or download: https://aka.ms/vs/17/release/vc_redist.x64.exe

Then restart your terminal.

## Troubleshooting

### Windows: "ani-tui" command does nothing (no output)

**This means you're missing Visual C++ Redistributable!**

1. Install it: https://aka.ms/vs/17/release/vc_redist.x64.exe
2. Restart your computer
3. Try again

### Windows: "ani-tui" command not found

If typing `ani-tui` doesn't work after installation:

1. **Open a NEW terminal window** (PATH changes require a fresh PowerShell/CMD session)
2. Try running the full path:
   ```powershell
   $env:LOCALAPPDATA\ani-tui\ani-tui.exe
   ```
3. Or run the diagnostic tool:
   ```powershell
   iwr -useb https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/windows/diagnose.ps1 | iex
   ```

### Video doesn't play

You must install **mpv** before using ani-tui:
- **Windows**: Download from [mpv.io](https://mpv.io/installation/) or use `winget install mpv`
- **macOS**: `brew install mpv`
- **Linux**: `sudo apt install mpv`

## License

MIT License - See LICENSE file for details