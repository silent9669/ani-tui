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

### Quick Install (Recommended)

Open **PowerShell** and run:

```powershell
iex (irm https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/install-windows.ps1)
```

> This installs ani-tui and all dependencies automatically via [Scoop](https://scoop.sh).

### Manual Install

```powershell
# 1. Install Scoop (if not installed)
irm get.scoop.sh | iex

# 2. Install dependencies
scoop bucket add extras
scoop install fzf chafa ani-cli mpv

# 3. Download ani-tui
mkdir "$env:USERPROFILE\.ani-tui\bin" -Force
@("ani-tui-core.ps1", "ani-tui.ps1") | % {
    irm "https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/$_" -OutFile "$env:USERPROFILE\.ani-tui\bin\$_"
}

# 4. Create launcher & add to PATH
"@echo off`npowershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File `"%~dp0ani-tui-core.ps1`" %*" | Out-File "$env:USERPROFILE\.ani-tui\bin\ani-tui.cmd" -Encoding ASCII
$p = [Environment]::GetEnvironmentVariable("PATH","User")
if ($p -notlike "*\.ani-tui\bin*") { [Environment]::SetEnvironmentVariable("PATH","$p;$env:USERPROFILE\.ani-tui\bin","User") }

# 5. Restart terminal, then run:
ani-tui
```

### Update

```powershell
# Re-download latest scripts
@("ani-tui-core.ps1", "ani-tui.ps1") | % {
    irm "https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/$_" -OutFile "$env:USERPROFILE\.ani-tui\bin\$_"
}
```

### Dependencies

| Package | Purpose | Required |
|---------|---------|:--------:|
| fzf | Fuzzy finder UI | ✅ |
| chafa | Image previews | ⭐ Optional |
| ani-cli | Video streaming | ⭐ Optional |
| mpv | Video player | ⭐ Optional |

```powershell
# Install all dependencies
scoop bucket add extras && scoop install fzf chafa ani-cli mpv
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