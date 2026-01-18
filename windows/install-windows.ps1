<#
.SYNOPSIS
    Full automated installer for ani-tui on Windows
.DESCRIPTION
    Installs ani-tui with all dependencies for full macOS feature parity:
    - fzf (fuzzy finder TUI)
    - chafa (terminal image previews)
    - ani-cli + mpv (streaming support)
#>
[CmdletBinding()]
param(
    [switch]$NoDeps
)

$ErrorActionPreference = "Stop"
$ScriptUrl = "https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/ani-tui.ps1"

function Write-Step { Write-Host "`n==== $args ====" -ForegroundColor Cyan }
function Write-Ok { Write-Host "  OK: $args" -ForegroundColor Green }
function Write-Err { Write-Host "  ERR: $args" -ForegroundColor Red }
function Write-Info { Write-Host "  INFO: $args" -ForegroundColor Yellow }

Write-Step "Starting ani-tui Installation"

# 1. Setup Directories
$InstallDir = "$env:USERPROFILE\.ani-tui"
$BinDir = "$InstallDir\bin"
$CacheDir = "$InstallDir\cache"
$ImagesDir = "$CacheDir\images"

foreach ($dir in @($BinDir, $CacheDir, $ImagesDir)) {
    if (-not (Test-Path $dir)) { 
        New-Item -ItemType Directory -Path $dir -Force | Out-Null 
    }
}

# Initialize history file
$HistoryFile = "$InstallDir\history.json"
if (-not (Test-Path $HistoryFile)) {
    "[]" | Out-File -Encoding UTF8 $HistoryFile
}

# 2. Download Main Script
Write-Step "Downloading ani-tui..."
try {
    Invoke-WebRequest $ScriptUrl -OutFile "$BinDir\ani-tui.ps1"
    Write-Ok "Downloaded to $BinDir\ani-tui.ps1"
} catch {
    Write-Err "Failed to download script. Check internet."; exit 1
}

# 3. Create CMD Shim (ani-tui.cmd)
$Shim = "$BinDir\ani-tui.cmd"
"@echo off`npowershell -NoProfile -ExecutionPolicy Bypass -File `"%~dp0ani-tui.ps1`" %*" | Out-File -Encoding ASCII $Shim
Write-Ok "Created shim at $Shim"

# 4. Add to PATH (Persistent)
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$BinDir*") {
    Write-Step "Adding to PATH..."
    [Environment]::SetEnvironmentVariable("PATH", "$UserPath;$BinDir", "User")
    $env:PATH += ";$BinDir"
    Write-Ok "Added to User PATH"
} else {
    Write-Ok "Already in PATH"
}

# 5. Dependency Installation via Scoop
if (-not $NoDeps) {
    Write-Step "Installing Dependencies..."
    
    # Check/Install Scoop
    if (-not (Get-Command "scoop" -ErrorAction SilentlyContinue)) {
        Write-Info "Scoop not found. Installing Scoop..."
        try {
            Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
            Invoke-RestMethod -Uri https://get.scoop.sh | Invoke-Expression
            Write-Ok "Scoop installed."
        } catch {
            Write-Err "Failed to install Scoop. You may need to run as Admin or check network."
        }
    } else {
        Write-Ok "Scoop detected."
    }

    # Install tools via Scoop
    if (Get-Command "scoop" -ErrorAction SilentlyContinue) {
        # Add extras bucket for ani-cli
        scoop bucket add extras 2>$null | Out-Null
        
        # Required: fzf (TUI framework)
        if (-not (Get-Command "fzf" -ErrorAction SilentlyContinue)) {
            Write-Info "Installing fzf (required for TUI)..."
            scoop install fzf
        } else { Write-Ok "fzf detected." }

        # Optional: chafa (image previews)
        if (-not (Get-Command "chafa" -ErrorAction SilentlyContinue)) {
            Write-Info "Installing chafa (for image previews)..."
            scoop install chafa
        } else { Write-Ok "chafa detected." }

        # Optional: ani-cli (streaming)
        if (-not (Get-Command "ani-cli" -ErrorAction SilentlyContinue)) {
            Write-Info "Installing ani-cli (for streaming)..."
            scoop install ani-cli
        } else { Write-Ok "ani-cli detected." }

        # Optional: mpv (video player)
        if (-not (Get-Command "mpv" -ErrorAction SilentlyContinue)) {
            Write-Info "Installing mpv (video player)..."
            scoop install mpv
        } else { Write-Ok "mpv detected." }
        
    } else {
        Write-Err "Scoop missing. Cannot install dependencies automatically."
        Write-Host ""
        Write-Host "Manual installation required:" -ForegroundColor Yellow
        Write-Host "  1. Install Scoop: https://scoop.sh" -ForegroundColor Gray
        Write-Host "  2. Run: scoop install fzf chafa ani-cli mpv" -ForegroundColor Gray
    }
}

Write-Step "Installation Complete!"
Write-Host ""
Write-Host "Restart your terminal, then run 'ani-tui' to start." -ForegroundColor Green
Write-Host ""
Write-Host "Dependencies Status:" -ForegroundColor Cyan
Write-Host "  fzf     - $(if (Get-Command 'fzf' -ErrorAction SilentlyContinue) { 'Installed (required)' } else { 'MISSING - run: scoop install fzf' })" -ForegroundColor $(if (Get-Command 'fzf' -ErrorAction SilentlyContinue) { 'Green' } else { 'Red' })
Write-Host "  chafa   - $(if (Get-Command 'chafa' -ErrorAction SilentlyContinue) { 'Installed (image previews)' } else { 'Optional - run: scoop install chafa' })" -ForegroundColor $(if (Get-Command 'chafa' -ErrorAction SilentlyContinue) { 'Green' } else { 'Yellow' })
Write-Host "  ani-cli - $(if (Get-Command 'ani-cli' -ErrorAction SilentlyContinue) { 'Installed (streaming)' } else { 'Optional - run: scoop install ani-cli' })" -ForegroundColor $(if (Get-Command 'ani-cli' -ErrorAction SilentlyContinue) { 'Green' } else { 'Yellow' })
Write-Host "  mpv     - $(if (Get-Command 'mpv' -ErrorAction SilentlyContinue) { 'Installed (video player)' } else { 'Optional - run: scoop install mpv' })" -ForegroundColor $(if (Get-Command 'mpv' -ErrorAction SilentlyContinue) { 'Green' } else { 'Yellow' })
Write-Host ""
