# Windows Installation Guide

## Quick Install

### PowerShell (Recommended)

Open PowerShell and run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://raw.githubusercontent.com/silent9669/ani-tui/main/packaging/windows/install-easy.ps1 | iex"
```

This will:
- Download the latest ani-tui release
- Install to `%LOCALAPPDATA%\ani-tui`
- Add to your PATH

### Manual Installation

1. Download the latest release from: https://github.com/silent9669/ani-tui/releases/latest
2. Download `ani-tui-windows-x86_64.zip`
3. Extract to `C:\Program Files\ani-tui` (or any folder)
4. Add that folder to your PATH environment variable

### Using Scoop

```powershell
scoop bucket add ani-tui https://github.com/silent9669/ani-tui
scoop install ani-tui
```

## Prerequisites

### Required: mpv

ani-tui requires mpv for video playback.

**Install mpv:**
- Download from: https://mpv.io/installation/
- Or use winget: `winget install mpv`
- Or use scoop: `scoop install mpv`

### Optional: chafa

For image previews (without this, anime covers won't display):
- Download from: https://hpjansson.org/chafa/
- Or use scoop: `scoop install chafa`

## Troubleshooting

### "ani-tui is not recognized as a command"

Restart your terminal after installation. The PATH changes require a new session.

### Video doesn't play

Make sure mpv is installed and in your PATH. Test by running `mpv --version` in Command Prompt.

### Images don't show

Install chafa for image preview support. Without it, the app will show placeholders.

## Uninstallation

To remove ani-tui:
1. Delete the installation folder (where you extracted it)
2. Remove the folder from your PATH environment variable