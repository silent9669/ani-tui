<#
.SYNOPSIS
    ani-tui Windows Installer v6.0
.DESCRIPTION
    Installs ani-tui and all dependencies for Windows.
#>

$ErrorActionPreference = "Stop"

$BaseUrl = "https://raw.githubusercontent.com/silent9669/ani-tui/master/windows"
$InstallDir = "$env:USERPROFILE\.ani-tui"
$BinDir = "$InstallDir\bin"

function Write-Step { Write-Host "`n=== $args ===" -ForegroundColor Cyan }
function Write-OK { Write-Host "  OK: $args" -ForegroundColor Green }
function Write-Err { Write-Host "  ERROR: $args" -ForegroundColor Red }

Write-Host ""
Write-Host "  ================================" -ForegroundColor Magenta
Write-Host "     ani-tui Installer v6.4" -ForegroundColor White
Write-Host "  ================================" -ForegroundColor Magenta
Write-Host ""

# 1. Create directories
Write-Step "Creating directories"
foreach ($d in @($InstallDir, $BinDir, "$InstallDir\cache", "$InstallDir\cache\images")) {
    if (!(Test-Path $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null }
}
Write-OK "Created $InstallDir"

# 2. Download scripts
Write-Step "Downloading ani-tui"
try {
    Invoke-WebRequest "$BaseUrl/ani-tui.ps1" -OutFile "$BinDir\ani-tui.ps1" -UseBasicParsing
    Invoke-WebRequest "$BaseUrl/ani-tui-core.ps1" -OutFile "$BinDir\ani-tui-core.ps1" -UseBasicParsing
    Write-OK "Downloaded scripts"
} catch {
    Write-Err "Failed to download. Check your internet connection."
    exit 1
}

# 3. Create launcher
Write-Step "Creating launcher"
$launcher = @"
@echo off
REM ani-tui v6.0 for Windows - Launcher
powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0ani-tui-core.ps1" %*
"@
$launcher | Out-File "$BinDir\ani-tui.cmd" -Encoding ASCII
Write-OK "Created ani-tui.cmd"

# 4. Add to PATH
Write-Step "Configuring PATH"
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$BinDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$userPath;$BinDir", "User")
    $env:PATH += ";$BinDir"
    Write-OK "Added to PATH"
} else {
    Write-OK "Already in PATH"
}

# 5. Initialize history
if (!(Test-Path "$InstallDir\history.json")) {
    "[]" | Out-File "$InstallDir\history.json" -Encoding UTF8
}

# 6. Install dependencies via Scoop
Write-Step "Installing dependencies"

# Check Scoop
if (!(Get-Command scoop -ErrorAction SilentlyContinue)) {
    Write-Host "  Scoop not found. Installing..." -ForegroundColor Yellow
    try {
        Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
        Invoke-RestMethod get.scoop.sh | Invoke-Expression
        Write-OK "Scoop installed"
    } catch {
        Write-Err "Could not install Scoop. Install manually: https://scoop.sh"
    }
}

if (Get-Command scoop -ErrorAction SilentlyContinue) {
    # Add extras bucket
    scoop bucket add extras 2>$null
    
    # Required: fzf
    if (!(Get-Command fzf -ErrorAction SilentlyContinue)) {
        Write-Host "  Installing fzf..." -ForegroundColor Yellow
        scoop install fzf
    } else { Write-OK "fzf ready" }
    
    # Optional: chafa (image preview)
    if (!(Get-Command chafa -ErrorAction SilentlyContinue)) {
        Write-Host "  Installing chafa..." -ForegroundColor Yellow
        scoop install chafa
    } else { Write-OK "chafa ready" }
    
    # Optional: ani-cli + mpv (streaming)
    if (!(Get-Command ani-cli -ErrorAction SilentlyContinue)) {
        Write-Host "  Installing ani-cli..." -ForegroundColor Yellow
        scoop install ani-cli
    } else { Write-OK "ani-cli ready" }
    
    if (!(Get-Command mpv -ErrorAction SilentlyContinue)) {
        Write-Host "  Installing mpv..." -ForegroundColor Yellow
        scoop install mpv
    } else { Write-OK "mpv ready" }
}

# Done
Write-Host ""
Write-Host "  ================================" -ForegroundColor Green
Write-Host "     Installation Complete!" -ForegroundColor White
Write-Host "  ================================" -ForegroundColor Green
Write-Host ""
Write-Host "  1. RESTART your terminal" -ForegroundColor Yellow
Write-Host "  2. Run: ani-tui" -ForegroundColor Yellow
Write-Host ""
Write-Host "  Dependencies:" -ForegroundColor Cyan
Write-Host "    fzf     - $(if(Get-Command fzf -ErrorAction SilentlyContinue){'Installed'}else{'MISSING: scoop install fzf'})"
Write-Host "    chafa   - $(if(Get-Command chafa -ErrorAction SilentlyContinue){'Installed'}else{'Optional: scoop install chafa'})"
Write-Host "    ani-cli - $(if(Get-Command ani-cli -ErrorAction SilentlyContinue){'Installed'}else{'Optional: scoop install ani-cli'})"
Write-Host "    mpv     - $(if(Get-Command mpv -ErrorAction SilentlyContinue){'Installed'}else{'Optional: scoop install mpv'})"
Write-Host ""
