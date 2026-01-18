<#
.SYNOPSIS
    ani-tui Windows Installer v6.5
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
Write-Host "     ani-tui Installer v6.5" -ForegroundColor White
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
REM ani-tui v6.5 for Windows - Launcher
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
    
    # Optional: viu (image preview) - download using curl.exe
    if (!(Get-Command viu -ErrorAction SilentlyContinue)) {
        Write-Host "  Installing viu (image preview)..." -ForegroundColor Yellow
        try {
            $viuDir = "$InstallDir\viu"
            if (!(Test-Path $viuDir)) { mkdir $viuDir -Force | Out-Null }
            $viuZip = "$viuDir\viu.zip"
            
            # Use curl.exe (built into Windows 10+) - handles redirects better
            $viuUrl = "https://github.com/atanunq/viu/releases/download/v1.5.1/viu-x86_64-pc-windows-msvc.zip"
            
            # Download with curl.exe
            $curlResult = & curl.exe -L -s -o $viuZip $viuUrl 2>&1
            
            if (Test-Path $viuZip) {
                # Extract
                Expand-Archive $viuZip -DestinationPath $viuDir -Force
                
                # Find viu.exe (might be in subfolder)
                $viuExe = Get-ChildItem $viuDir -Filter "viu.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
                if ($viuExe) {
                    Copy-Item $viuExe.FullName "$BinDir\viu.exe" -Force
                    Write-OK "viu installed"
                } else {
                    # Maybe viu.exe is directly in zip
                    if (Test-Path "$viuDir\viu.exe") {
                        Copy-Item "$viuDir\viu.exe" "$BinDir\viu.exe" -Force
                        Write-OK "viu installed"
                    } else {
                        throw "viu.exe not found in archive"
                    }
                }
                Remove-Item $viuZip -Force -ErrorAction SilentlyContinue
            } else {
                throw "Download failed"
            }
        } catch {
            Write-Host "  Could not install viu automatically" -ForegroundColor DarkGray
            Write-Host ""
            Write-Host "  Please install viu manually:" -ForegroundColor Yellow
            Write-Host "    1. Download: https://github.com/atanunq/viu/releases/download/v1.5.1/viu-x86_64-pc-windows-msvc.zip" -ForegroundColor Cyan
            Write-Host "    2. Extract viu.exe" -ForegroundColor Cyan
            Write-Host "    3. Copy to: $BinDir\viu.exe" -ForegroundColor Cyan
            Write-Host ""
        }
    } else { Write-OK "viu ready" }
    
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
Write-Host "    viu     - $(if(Get-Command viu -ErrorAction SilentlyContinue){'Installed'}else{'Optional: Download from github.com/atanunq/viu/releases'})"
Write-Host "    ani-cli - $(if(Get-Command ani-cli -ErrorAction SilentlyContinue){'Installed'}else{'Optional: scoop install ani-cli'})"
Write-Host "    mpv     - $(if(Get-Command mpv -ErrorAction SilentlyContinue){'Installed'}else{'Optional: scoop install mpv'})"
Write-Host ""
