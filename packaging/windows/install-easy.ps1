# Windows Easy Installer for ani-tui v2
# Improved installer with better PATH handling and diagnostics

param(
    [string]$InstallDir = "$env:LOCALAPPDATA\ani-tui"
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Installing ani-tui..." -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if ani-tui is already installed
$existingPath = Get-Command ani-tui -ErrorAction SilentlyContinue
if ($existingPath) {
    Write-Host "ani-tui is already installed at: $($existingPath.Source)" -ForegroundColor Yellow
    $response = Read-Host "Do you want to reinstall? (y/N)"
    if ($response -ne 'y' -and $response -ne 'Y') {
        Write-Host "Installation cancelled." -ForegroundColor Yellow
        exit 0
    }
}

# Create install directory
Write-Host "Creating installation directory..." -ForegroundColor Gray
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

# Download latest release
Write-Host "Downloading ani-tui..." -ForegroundColor Gray
$releaseUrl = "https://github.com/silent9669/ani-tui/releases/latest/download/ani-tui-windows-x86_64.zip"
$zipPath = "$env:TEMP\ani-tui-install.zip"

try {
    Invoke-WebRequest -Uri $releaseUrl -OutFile $zipPath -UseBasicParsing
    Write-Host "Download complete!" -ForegroundColor Green
} catch {
    Write-Host "Error downloading ani-tui: $_" -ForegroundColor Red
    exit 1
}

# Extract
Write-Host "Extracting files..." -ForegroundColor Gray
Expand-Archive -Path $zipPath -DestinationPath $InstallDir -Force
Remove-Item $zipPath -Force

# Verify binary exists
$binaryPath = Join-Path $InstallDir "ani-tui.exe"
if (-not (Test-Path $binaryPath)) {
    Write-Host "Error: ani-tui.exe not found after extraction!" -ForegroundColor Red
    Write-Host "Expected at: $binaryPath" -ForegroundColor Red
    exit 1
}

Write-Host "Binary found at: $binaryPath" -ForegroundColor Green

# Add to PATH
Write-Host "Adding to PATH..." -ForegroundColor Gray
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$InstallDir", "User")
    Write-Host "Added to PATH successfully!" -ForegroundColor Green
} else {
    Write-Host "Already in PATH" -ForegroundColor Gray
}

# Also update current session PATH
$env:PATH = "$env:PATH;$InstallDir"

# Create wrapper batch file for easy access
$wrapperPath = Join-Path $InstallDir "ani-tui.bat"
$wrapperContent = @"
@echo off
"$binaryPath" %*
"@
Set-Content -Path $wrapperPath -Value $wrapperContent

# Check for dependencies
Write-Host ""
Write-Host "Checking dependencies..." -ForegroundColor Gray

# Check for chafa
$chafa = Get-Command chafa -ErrorAction SilentlyContinue
if (-not $chafa) {
    Write-Host "  chafa not found. Image previews will not work." -ForegroundColor Yellow
}

# Check for mpv
$mpv = Get-Command mpv -ErrorAction SilentlyContinue
if (-not $mpv) {
    Write-Host "  mpv not found. Video playback will not work." -ForegroundColor Yellow
    Write-Host "" -ForegroundColor White
    Write-Host "IMPORTANT: You need to install mpv before using ani-tui!" -ForegroundColor Red
    Write-Host "Download from: https://mpv.io/installation/" -ForegroundColor Cyan
}

# Test the binary
Write-Host ""
Write-Host "Testing installation..." -ForegroundColor Gray
try {
    $version = & $binaryPath --version 2>&1
    if ($version) {
        Write-Host "✓ ani-tui is working! Version: $version" -ForegroundColor Green
    }
} catch {
    Write-Host "Warning: Could not verify ani-tui. Error: $_" -ForegroundColor Yellow
}

# Installation complete
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "Installation Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Installed to: $InstallDir" -ForegroundColor White
Write-Host ""
Write-Host "Usage:" -ForegroundColor Cyan
Write-Host "  ani-tui              - Start the app"
Write-Host "  ani-tui -q ""naruto"" - Search immediately"
Write-Host ""
Write-Host "Getting Started:" -ForegroundColor Cyan
Write-Host "  1. Open a NEW terminal window (important!)"
Write-Host "  2. Type 'ani-tui' to launch"
Write-Host "  3. Press Shift+S to search"
Write-Host ""

if (-not $mpv) {
    Write-Host "⚠️  IMPORTANT: Install mpv first or video won't play!" -ForegroundColor Red
    Write-Host "   Download: https://mpv.io/installation/" -ForegroundColor Yellow
    Write-Host ""
}

Write-Host "Press Enter to exit..."
Read-Host