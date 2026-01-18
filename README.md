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

### Windows

**Option 1: PowerShell (Recommended)**
1. Open **PowerShell** (Search for "PowerShell" in Start Menu)
2. Run the following commands:
```powershell
git clone https://github.com/silent9669/ani-tui.git "$env:USERPROFILE\.ani-tui"
& "$env:USERPROFILE\.ani-tui\windows\ani-tui.ps1"
```

**Option 2: Command Prompt (cmd)**
1. Open **Command Prompt** (cmd.exe)
2. Run the following commands:
```cmd
git clone https://github.com/silent9669/ani-tui.git "%USERPROFILE%\.ani-tui"
powershell -ExecutionPolicy Bypass -File "%USERPROFILE%\.ani-tui\windows\ani-tui.ps1"
```

**Option 3: Simple Install (PowerShell)**
Paste this entire line into **PowerShell**:
```powershell
mkdir "$env:USERPROFILE\.ani-tui" -Force; Invoke-WebRequest "https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/ani-tui.ps1" -OutFile "$env:USERPROFILE\.ani-tui\ani-tui.ps1"; powershell -ExecutionPolicy Bypass -File "$env:USERPROFILE\.ani-tui\ani-tui.ps1"
```

**Option 4: Curl Install (Command Prompt)**
Paste this into **cmd.exe**:
```cmd
if not exist "%USERPROFILE%\.ani-tui" mkdir "%USERPROFILE%\.ani-tui" & curl -L -o "%USERPROFILE%\.ani-tui\ani-tui.ps1" "https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/ani-tui.ps1" & powershell -ExecutionPolicy Bypass -File "%USERPROFILE%\.ani-tui\ani-tui.ps1"
```

### Windows Troubleshooting
If you see errors like "destination path already exists" or "Unexpected token", run this **Clean Reinstall** command in PowerShell:
```powershell
Remove-Item "$env:USERPROFILE\.ani-tui" -Recurse -Force -ErrorAction SilentlyContinue; mkdir "$env:USERPROFILE\.ani-tui" -Force; Invoke-WebRequest "https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/ani-tui.ps1" -OutFile "$env:USERPROFILE\.ani-tui\ani-tui.ps1"; powershell -ExecutionPolicy Bypass -File "$env:USERPROFILE\.ani-tui\ani-tui.ps1"
```

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

### Required
| Package | Purpose | Install |
|---------|---------|---------|
| curl | API requests | `brew install curl` |
| jq | JSON parsing | `brew install jq` |
| fzf | Fuzzy finder UI | `brew install fzf` |

### Recommended
| Package | Purpose | Install |
|---------|---------|---------|
| chafa | Image previews | `brew install chafa` |
| mpv | Video playback | `brew install mpv` |
| iina | Video playback (alternative) | `brew install --cask iina` |

**Install all at once:**
```bash
brew install curl jq fzf chafa mpv
```

---

## Data Locations

| Type | Path |
|------|------|
| Cache | `~/.cache/ani-tui/` |
| Images | `~/.cache/ani-tui/images/` |
| History | `~/.local/share/ani-tui/history.json` |

---

## Uninstall

```bash
# Homebrew
brew uninstall ani-tui
brew untap silent9669/tap

# Manual
rm -rf ~/.local/share/ani-tui
rm -rf ~/.cache/ani-tui
rm /usr/local/bin/ani-tui
```

---

## License

GPL-3.0 — Based on [ani-cli](https://github.com/pystardust/ani-cli)