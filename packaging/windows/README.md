# Windows Installation Guide

## 🌟 Complete Auto-Installer (Recommended)

This installer automatically downloads and installs:
- **mpv** (required for video playback)
- **chafa** (optional for image previews)
- **ani-tui**

### PowerShell Method

Open PowerShell and run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/windows/install-complete.ps1 | iex"
```

### Batch File Method (Double-Click)

1. Download [install-all.bat](https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/windows/install-all.bat)
2. Double-click the downloaded file
3. Follow the prompts

### What This Installs

| Software | Purpose | Status |
|----------|---------|--------|
| mpv | Video playback | **REQUIRED** |
| chafa | Image previews | Optional |
| ani-tui | Main app | **REQUIRED** |

All software is installed to: `%USERPROFILE%\ani-tui`

## Other Installation Methods

### Manual Download

1. Download the latest release: https://github.com/silent9669/ani-tui/releases/latest
2. Download `ani-tui-windows-x86_64.zip`
3. Extract to a folder
4. Add that folder to your PATH

### Using Scoop

```powershell
scoop bucket add ani-tui https://github.com/silent9669/ani-tui
scoop install ani-tui
scoop install mpv
```

## Important Notes

### You MUST Open a NEW Terminal

After installation, **close your current terminal and open a new one**. The PATH changes require a fresh terminal session.

### Test Your Installation

After opening a new terminal, test with:
```powershell
ani-tui --version
```

### If "ani-tui" Command Not Found

Run the diagnostic tool:
```powershell
iwr -useb https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/windows/diagnose.ps1 | iex
```

Or use the full path:
```powershell
$env:USERPROFILE\ani-tui\ani-tui.exe
```

## Troubleshooting

### "ani-tui is not recognized as a command"

1. Open a NEW terminal window (important!)
2. Try again
3. If still not working, reinstall with the complete installer above

### Video doesn't play

The complete installer should install mpv automatically. If not:
```powershell
winget install mpv
```

### Images don't show

The complete installer should install chafa automatically. If not, it's optional - the app will work without it.

## Uninstallation

To remove ani-tui and all dependencies:

1. Delete the installation folder: `%USERPROFILE%\ani-tui`
2. Remove from PATH:
   - Open System Properties → Environment Variables
   - Edit "Path" under User variables
   - Remove the ani-tui folder path