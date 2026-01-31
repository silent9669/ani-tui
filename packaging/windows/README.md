# Windows Installation Guide for ani-tui

## Quick Install (Recommended)

### Option 1: One-Line PowerShell Command
Copy and paste this command into PowerShell (Run as Administrator):

```powershell
powershell -ExecutionPolicy Bypass -Command "iwr -useb https://raw.githubusercontent.com/silent9669/ani-tui/main/packaging/windows/install-easy.ps1 | iex"
```

### Option 2: Download and Run Batch File
1. Download [install.bat](https://raw.githubusercontent.com/silent9669/ani-tui/main/packaging/windows/install.bat)
2. Double-click the downloaded file
3. Follow the prompts

### Option 3: Manual Installation
1. Download the latest release: [ani-tui-windows-x86_64.zip](https://github.com/silent9669/ani-tui/releases/latest/download/ani-tui-windows-x86_64.zip)
2. Extract the zip file to a folder (e.g., `C:\Program Files\ani-tui`)
3. Add that folder to your PATH environment variable
4. Open Command Prompt or PowerShell and type `ani-tui`

## Prerequisites

Before running ani-tui, you'll need:

### Required:
- **mpv** - For video playback
  - Download from: https://mpv.io/installation/
  - Or install with winget: `winget install mpv`

### Optional:
- **chafa** - For image previews (without this, images won't display)
  - Download from: https://hpjansson.org/chafa/

## Installing Prerequisites

### Using winget (Windows 10/11):
```powershell
winget install mpv
winget install chafa
```

### Manual Download:
1. **mpv**: https://mpv.io/installation/ → Download Windows build → Extract to `C:\Program Files\mpv`
2. **chafa**: https://hpjansson.org/chafa/ → Download Windows build → Extract to `C:\Program Files\chafa`
3. Add both directories to your PATH

## Usage

Once installed, open Command Prompt or PowerShell and run:

```cmd
ani-tui
```

Or start with a search:

```cmd
ani-tui -q "Attack on Titan"
```

## Troubleshooting

### "ani-tui is not recognized as a command"
- Restart your terminal after installation
- Make sure the installation directory is in your PATH

### "mpv not found"
- Install mpv from https://mpv.io/installation/
- Make sure mpv is in your PATH

### Video doesn't play
- Ensure mpv is properly installed
- Try running `mpv --version` to verify

## Uninstallation

To uninstall:
1. Delete the installation folder (e.g., `C:\Program Files\ani-tui`)
2. Remove the folder from your PATH environment variable

## Support

For issues and support, visit: https://github.com/silent9669/ani-tui/issues