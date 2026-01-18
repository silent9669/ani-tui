# ani-tui

🎬 **Anime TUI** with image previews and terminal streaming.

![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-brightgreen)
![License](https://img.shields.io/badge/license-GPL--3.0-blue)

## Features

- 🔍 **Real-time fuzzy search** - Type to search anime instantly
- 🖼️ **Cover image previews** - Beautiful anime covers in your terminal
- ▶️ **Stream directly** - Play in mpv/iina without leaving the terminal
- 📺 **Watch history** - Track your progress across anime
- ⌨️ **Keyboard-driven** - Navigate entirely with keyboard shortcuts

---

## Installation

### macOS (Homebrew) — Recommended

```bash
# Add the tap and install
brew tap silent9669/tap
brew install ani-tui

# Install recommended dependencies for full experience
brew install chafa mpv
```

That's it! Now run `ani-tui` from any terminal.

### macOS (Manual)

```bash
# 1. Install dependencies
brew install curl jq fzf chafa mpv

# 2. Clone the repository
git clone https://github.com/silent9669/ani-tui.git ~/.local/share/ani-tui
cd ~/.local/share/ani-tui

# 3. Make executable and link
chmod +x macos/ani-tui
ln -sf ~/.local/share/ani-tui/macos/ani-tui /usr/local/bin/ani-tui

# 4. Run
ani-tui
```

---

## Windows Installation

### Quick Install (Recommended)

Open **PowerShell** and run:

```powershell
Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
iex (irm https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/install-windows.ps1)
```

This will:
- Install ani-tui and add to PATH
- Install [Scoop](https://scoop.sh) if not present
- Install all dependencies: `fzf`, `chafa`, `ani-cli`, `mpv`

**Restart your terminal**, then run:
```powershell
ani-tui
```

### Manual Install

If you prefer manual installation:

```powershell
# 1. Install Scoop (package manager)
Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
irm get.scoop.sh | iex

# 2. Install dependencies
scoop bucket add extras
scoop install fzf chafa ani-cli mpv

# 3. Download ani-tui
mkdir "$env:USERPROFILE\.ani-tui\bin" -Force
Invoke-WebRequest "https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/ani-tui.ps1" -OutFile "$env:USERPROFILE\.ani-tui\bin\ani-tui.ps1"

# 4. Create launcher batch file
'@echo off`npowershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0ani-tui.ps1" %*' | Out-File "$env:USERPROFILE\.ani-tui\bin\ani-tui.cmd" -Encoding ASCII

# 5. Add to PATH
$path = [Environment]::GetEnvironmentVariable("PATH", "User")
[Environment]::SetEnvironmentVariable("PATH", "$path;$env:USERPROFILE\.ani-tui\bin", "User")

# 6. Restart terminal and run
ani-tui
```

### Windows Dependencies

| Package | Purpose | Required | Install |
|---------|---------|----------|---------|
| fzf | Fuzzy finder TUI | ✅ Yes | `scoop install fzf` |
| chafa | Image previews | Optional | `scoop install chafa` |
| ani-cli | Streaming | Optional | `scoop install ani-cli` |
| mpv | Video player | Optional | `scoop install mpv` |

**Install all at once:**
```powershell
scoop bucket add extras
scoop install fzf chafa ani-cli mpv
```

### Windows Troubleshooting

**Clean Reinstall:**
```powershell
Remove-Item "$env:USERPROFILE\.ani-tui" -Recurse -Force -ErrorAction SilentlyContinue
iex (irm https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/install-windows.ps1)
```

**Common Issues:**
- **"fzf not found"** → Run: `scoop install fzf`
- **"ani-cli not found"** → Run: `scoop bucket add extras; scoop install ani-cli mpv`
- **No image previews** → Run: `scoop install chafa`
- **Script won't run** → Run: `Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force`

---

## Usage

```bash
ani-tui
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Type | Search anime in real-time |
| ↑ / ↓ | Navigate list |
| Enter | Select anime / Play episode |
| Ctrl-D | Delete from watch history |
| Esc | Go back / Quit |

### Workflow

1. **Start** → Shows your watch history
2. **Type** → Search for new anime
3. **Select** → Pick an anime
4. **Choose episode** → Starts streaming in mpv

---

## Dependencies

### macOS

| Package | Purpose | Install |
|---------|---------|---------|
| curl | API requests | `brew install curl` |
| jq | JSON parsing | `brew install jq` |
| fzf | Fuzzy finder UI | `brew install fzf` |
| chafa | Image previews | `brew install chafa` |
| mpv | Video playback | `brew install mpv` |

**Install all:**
```bash
brew install curl jq fzf chafa mpv
```

### Windows

See [Windows Dependencies](#windows-dependencies) above.

---

## Data Locations

| Type | macOS | Windows |
|------|-------|---------|
| Cache | `~/.cache/ani-tui/` | `%USERPROFILE%\.ani-tui\cache\` |
| Images | `~/.cache/ani-tui/images/` | `%USERPROFILE%\.ani-tui\cache\images\` |
| History | `~/.local/share/ani-tui/history.json` | `%USERPROFILE%\.ani-tui\history.json` |

---

## Uninstall

### macOS
```bash
# Homebrew
brew uninstall ani-tui
brew untap silent9669/tap

# Manual
rm -rf ~/.local/share/ani-tui
rm -rf ~/.cache/ani-tui
rm /usr/local/bin/ani-tui
```

### Windows
```powershell
# Remove ani-tui
Remove-Item "$env:USERPROFILE\.ani-tui" -Recurse -Force

# Optional: Remove from PATH manually in System Settings
# Optional: Uninstall deps
scoop uninstall ani-cli mpv chafa fzf
```

---

## License

GPL-3.0 — Based on [ani-cli](https://github.com/pystardust/ani-cli)