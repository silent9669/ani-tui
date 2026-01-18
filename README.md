# ani-tui

🎬 Anime TUI with image previews and terminal streaming via mpv/iina.

![License](https://img.shields.io/badge/license-GPL--3.0-blue)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-brightgreen)

## Features

- 🔍 Real-time fuzzy search
- 🖼️ Cover image previews
- ▶️ Stream directly in mpv/iina
- 📺 Watch history tracking

---

## Installation

### macOS (Homebrew)

**One-liner install:**
```bash
brew tap silent9669/ani-tui && brew install ani-tui
```

**Or manual install:**
```bash
# 1. Install Homebrew (skip if already installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# 2. Install all dependencies
brew install curl jq fzf chafa mpv

# 3. Clone repository
git clone https://github.com/silent9669/ani-tui.git ~/.local/share/ani-tui
cd ~/.local/share/ani-tui

# 4. Make executable and create symlink
chmod +x macos/ani-tui
ln -sf ~/.local/share/ani-tui/macos/ani-tui /usr/local/bin/ani-tui

# 5. Run
ani-tui
```

---

### Windows

**Option 1: Git clone (recommended)**
```powershell
git clone https://github.com/silent9669/ani-tui.git $env:USERPROFILE\.ani-tui
powershell -ExecutionPolicy Bypass -File "$env:USERPROFILE\.ani-tui\windows\ani-tui.ps1"
```

**Option 2: One-liner with curl**
```powershell
mkdir $env:USERPROFILE\.ani-tui -Force; Invoke-WebRequest -Uri "https://raw.githubusercontent.com/silent9669/ani-tui/main/windows/ani-tui.ps1" -OutFile "$env:USERPROFILE\.ani-tui\ani-tui.ps1"; powershell -ExecutionPolicy Bypass -File "$env:USERPROFILE\.ani-tui\ani-tui.ps1"
```

---

## Usage

```bash
ani-tui
```

| Key | Action |
|-----|--------|
| Type | Search anime |
| ↑↓ | Navigate |
| Enter | Select / Play |
| Esc | Back / Quit |

---

## Dependencies

### macOS
| Package | Install |
|---------|---------|
| curl | `brew install curl` |
| jq | `brew install jq` |
| fzf | `brew install fzf` |
| chafa | `brew install chafa` |
| mpv | `brew install mpv` |

**Install all:**
```bash
brew install curl jq fzf chafa mpv
```

### Windows
- PowerShell 5.1+ (built-in)

---

## Publishing to Homebrew

To submit to Homebrew:

1. Create a GitHub release with tag `v1.0.0`
2. Get the tarball SHA256:
   ```bash
   curl -sL https://github.com/silent9669/ani-tui/archive/refs/tags/v1.0.0.tar.gz | shasum -a 256
   ```
3. Update `Formula/ani-tui.rb` with the SHA256
4. Fork [homebrew-core](https://github.com/Homebrew/homebrew-core)
5. Add formula and submit PR

Or create your own tap:
```bash
brew tap-new silent9669/homebrew-ani-tui
cp Formula/ani-tui.rb $(brew --repo silent9669/homebrew-ani-tui)/Formula/
cd $(brew --repo silent9669/homebrew-ani-tui)
git add . && git commit -m "Add ani-tui formula"
git push
```

---

## License

GPL-3.0
