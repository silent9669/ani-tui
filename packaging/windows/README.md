# Windows Installation Guide

## Recommended Installer

Open PowerShell and run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://github.com/silent9669/ani-tui/releases/latest/download/install.ps1 -OutFile install.ps1; .\install.ps1"
```

The installer:

- installs ani-tui to `%LOCALAPPDATA%\ani-tui`
- verifies Visual C++ Redistributable when winget is available
- installs or locates `mpv` for video playback
- falls back to a portable shinchiro mpv build under `%LOCALAPPDATA%\ani-tui\tools\mpv`
- sets `ANI_TUI_PLAYER` to the resolved `mpv.exe`
- adds ani-tui to the user PATH

Open a new terminal after installation and run:

```powershell
ani-tui
```

## Troubleshooting

Run the diagnostic script:

```powershell
iwr -useb https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/windows/diagnose.ps1 | iex
```

If video playback does not open an mpv window, inspect:

```powershell
$env:TEMP\ani-tui-mpv.log
```

You can also force a specific player:

```powershell
[Environment]::SetEnvironmentVariable("ANI_TUI_PLAYER", "C:\path\to\mpv.exe", "User")
```

## Legacy Scripts

`install-complete.ps1`, `install-easy.ps1`, `install-all.bat`, and `install.bat` are compatibility wrappers. New documentation and releases should point to `install.ps1`.
