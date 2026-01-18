# ani-tui

🎬 **Anime TUI** with image previews and terminal streaming.

![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-brightgreen)
![License](https://img.shields.io/badge/license-GPL--3.0-blue)

## Features

- 🔍 **Real-time search** - Type to search anime instantly
- 🖼️ **Image previews** - Anime covers in your terminal (via chafa)
- ▶️ **Stream directly** - Play in mpv without leaving terminal
- 📺 **Watch history** - Track progress across anime
- ⌨️ **Keyboard-driven** - Navigate with keyboard shortcuts

---

## macOS Installation

### Homebrew (Recommended)

```bash
brew tap silent9669/tap
brew install ani-tui
brew install chafa mpv  # for image previews and streaming
```

### Manual

```bash
brew install curl jq fzf chafa mpv
git clone https://github.com/silent9669/ani-tui.git ~/.local/share/ani-tui
chmod +x ~/.local/share/ani-tui/macos/ani-tui
ln -sf ~/.local/share/ani-tui/macos/ani-tui /usr/local/bin/ani-tui
```

---

## Windows Installation

### One-Line Install (PowerShell)

```powershell
Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force; iex (irm https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/install-windows.ps1)
```

### Using curl (Command Prompt)

```cmd
curl -L -o "%TEMP%\install.ps1" https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/install-windows.ps1 && powershell -ExecutionPolicy Bypass -File "%TEMP%\install.ps1"
```

### Manual Install

```powershell
# 1. Install Scoop (package manager)
irm get.scoop.sh | iex

# 2. Install dependencies
scoop bucket add extras
scoop install fzf chafa ani-cli mpv

# 3. Download ani-tui
mkdir "$env:USERPROFILE\.ani-tui\bin" -Force
irm https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/ani-tui.ps1 -OutFile "$env:USERPROFILE\.ani-tui\bin\ani-tui.ps1"
irm https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/ani-tui-core.ps1 -OutFile "$env:USERPROFILE\.ani-tui\bin\ani-tui-core.ps1"

# 4. Create launcher
"@echo off`npowershell -NoProfile -ExecutionPolicy Bypass -File `"%~dp0ani-tui.ps1`" %*" | Out-File "$env:USERPROFILE\.ani-tui\bin\ani-tui.cmd" -Encoding ASCII

# 5. Add to PATH
$p = [Environment]::GetEnvironmentVariable("PATH","User")
[Environment]::SetEnvironmentVariable("PATH","$p;$env:USERPROFILE\.ani-tui\bin","User")

# 6. Restart terminal, then run:
ani-tui
```

### Windows Dependencies

| Package | Purpose | Required | Install |
|---------|---------|:--------:|---------|
| fzf | Fuzzy finder UI | ✅ | `scoop install fzf` |
| chafa | Image previews | ⭐ | `scoop install chafa` |
| ani-cli | Video streaming | ⭐ | `scoop install ani-cli` |
| mpv | Video player | ⭐ | `scoop install mpv` |

```powershell
# Install all at once
scoop bucket add extras
scoop install fzf chafa ani-cli mpv
```

### Troubleshooting

**Clean reinstall:**
```powershell
Remove-Item "$env:USERPROFILE\.ani-tui" -Recurse -Force -ErrorAction SilentlyContinue
iex (irm https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/install-windows.ps1)
```

---

## Usage

```bash
ani-tui
```

### Controls

| Key | Action |
|-----|--------|
| Type | Search anime (real-time) |
| ↑ / ↓ | Navigate |
| Enter | Select / Play |
| Ctrl-D | Delete from history |
| Esc | Back / Quit |

### Workflow

1. Launch `ani-tui`
2. Type to search or select from history
3. Pick an anime
4. Choose episode
5. Watch in mpv

---

## Data Locations

| | macOS | Windows |
|---|-------|---------|
| Cache | `~/.cache/ani-tui/` | `%USERPROFILE%\.ani-tui\cache\` |
| History | `~/.local/share/ani-tui/history.json` | `%USERPROFILE%\.ani-tui\history.json` |

---

## Uninstall

### macOS
```bash
brew uninstall ani-tui && brew untap silent9669/tap
rm -rf ~/.cache/ani-tui ~/.local/share/ani-tui
```

### Windows
```powershell
Remove-Item "$env:USERPROFILE\.ani-tui" -Recurse -Force
# Remove from PATH manually in System Settings
```

---

## License

GPL-3.0 — Based on [ani-cli](https://github.com/pystardust/ani-cli)