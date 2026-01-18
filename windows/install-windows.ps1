<#
.SYNOPSIS
    Full automated installer for ani-tui on Windows
.DESCRIPTION
    Installs ani-tui, and optionally installs Scoop + ani-cli + mpv for streaming support.
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

Write-Step "Starting ani-tui Installation"

# 1. Setup Directories
$InstallDir = "$env:USERPROFILE\.ani-tui"
$BinDir = "$InstallDir\bin"
if (-not (Test-Path $BinDir)) { New-Item -ItemType Directory -Path $BinDir -Force | Out-Null }

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
"@echo off`npowershell -ExecutionPolicy Bypass -NoLogo -File `"%~dp0ani-tui.ps1`" %*" | Out-File -Encoding ASCII $Shim
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

# 5. Dependency Check & Install (Scoop -> ani-cli, mpv)
if (-not $NoDeps) {
    Write-Step "Checking Dependencies (for streaming)..."
    
    # Check Scoop
    if (-not (Get-Command "scoop" -ErrorAction SilentlyContinue)) {
        Write-Host "  Scoop not found. Installing Scoop..." -ForegroundColor Yellow
        try {
            Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
            Invoke-RestMethod -Uri https://get.scoop.sh | Invoke-Expression
            Write-Ok "Scoop installed."
        } catch {
            Write-Err "Failed to install Scoop. You may need to run as Admin or check network."
        }
    }

    # Install tools via Scoop
    if (Get-Command "scoop" -ErrorAction SilentlyContinue) {
        scoop bucket add extras | Out-Null
        
        if (-not (Get-Command "ani-cli" -ErrorAction SilentlyContinue)) {
            Write-Host "  Installing ani-cli (via Scoop)..." -ForegroundColor Yellow
            scoop install ani-cli
        } else { Write-Ok "ani-cli detected." }

        if (-not (Get-Command "mpv" -ErrorAction SilentlyContinue)) {
            Write-Host "  Installing mpv (via Scoop)..." -ForegroundColor Yellow
            scoop install mpv
        } else { Write-Ok "mpv detected." }
    } else {
        Write-Err "Scoop missing. Cannot install streaming dependencies automatically."
    }
}

Write-Step "Installation Complete!"
Write-Host "Restart your terminal to use 'ani-tui' command." -ForegroundColor Green
Write-Host "NOTE: To stream video, 'ani-cli' and 'mpv' are required." -ForegroundColor Gray
