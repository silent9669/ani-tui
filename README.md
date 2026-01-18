# ani-tui

🎬 **Anime TUI** with image previews and terminal streaming.

![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-brightgreen)
![Version](https://img.shields.io/badge/version-6.5-blue)
![License](https://img.shields.io/badge/license-GPL--3.0-blue)

## Features

- 🔍 **Real-time search** - Type to search anime instantly
- 🖼️ **Image previews** - Anime covers in your terminal (via viu)
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

### Quick Install (One Command)

Open **PowerShell** and run:

```powershell
iex (irm https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/install-windows.ps1)
```

This installs everything automatically including:
- ✅ ani-tui scripts
- ✅ Scoop package manager
- ✅ fzf (UI)
- ✅ viu (image preview)
- ✅ ani-cli + mpv (streaming)
- ✅ Git (required by ani-cli)

### Update

```powershell
iex (irm https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/install-windows.ps1)
```

### Manual Install

```powershell
# 1. Install Scoop
irm get.scoop.sh | iex

# 2. Install dependencies
scoop bucket add extras
scoop install git fzf ani-cli mpv

# 3. Install viu (image preview)
$viuUrl = "https://github.com/atanunq/viu/releases/latest/download/viu-x86_64-pc-windows-msvc.zip"
Invoke-WebRequest $viuUrl -OutFile "$env:TEMP\viu.zip"
Expand-Archive "$env:TEMP\viu.zip" -DestinationPath "$env:USERPROFILE\.ani-tui\bin" -Force

# 4. Download ani-tui
mkdir "$env:USERPROFILE\.ani-tui\bin" -Force
@("ani-tui-core.ps1", "ani-tui.ps1") | % {
    irm "https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/$_" -OutFile "$env:USERPROFILE\.ani-tui\bin\$_"
}

# 5. Create launcher & add to PATH
"@echo off`npowershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File `"%~dp0ani-tui-core.ps1`" %*" | Out-File "$env:USERPROFILE\.ani-tui\bin\ani-tui.cmd" -Encoding ASCII
$p = [Environment]::GetEnvironmentVariable("PATH","User")
if ($p -notlike "*\.ani-tui\bin*") { [Environment]::SetEnvironmentVariable("PATH","$p;$env:USERPROFILE\.ani-tui\bin","User") }

# 6. Restart terminal, then run:
ani-tui
```

### Troubleshooting

<details>
<summary><b>Image preview not working</b></summary>

```powershell
# Clear cache and restart
Remove-Item "$env:USERPROFILE\.ani-tui\cache" -Recurse -Force

# Verify viu is installed
viu --version
```
</details>

<details>
<summary><b>Streaming not working (WSL error)</b></summary>

```powershell
# ani-cli requires Git Bash
scoop install git

# Verify
Test-Path "$env:ProgramFiles\Git\bin\bash.exe"
```
</details>

<details>
<summary><b>Clean reinstall</b></summary>

```powershell
Remove-Item "$env:USERPROFILE\.ani-tui" -Recurse -Force -ErrorAction SilentlyContinue
iex (irm https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/install-windows.ps1)
```
</details>

---

## Usage

```bash
ani-tui
```

### Controls

| Key | Action |
|-----|--------|
| Type | Search anime |
| ↑ / ↓ | Navigate |
| Enter | Select / Play |
| Ctrl-D | Delete from history |
| Esc | Back / Quit |

### Workflow

1. Launch `ani-tui`
2. Type to search or browse history
3. Select anime → see preview
4. Choose episode → watch in mpv

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
```

---

## License

GPL-3.0 — Based on [ani-cli](https://github.com/pystardust/ani-cli)