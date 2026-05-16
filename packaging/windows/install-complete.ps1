# Complete Windows Installer for ani-tui v3
# Installs ani-tui with ALL dependencies including Visual C++ Redistributable

param(
    [string]$InstallDir = "$env:LOCALAPPDATA\ani-tui"
)

$ErrorActionPreference = "Continue"

function Write-Status {
    param($Message, $Color = "White")
    Write-Host $Message -ForegroundColor $Color
}

function Test-Command {
    param($Command)
    $null -ne (Get-Command $Command -ErrorAction SilentlyContinue)
}

function Show-AsciiBanner {
    Write-Host ''
    Write-Host '  /$$$$$$  /$$   /$$ /$$$$$$    /$$$$$$$$ /$$   /$$ /$$$$$$' -ForegroundColor Cyan
    Write-Host ' /$$__  $$| $$$ | $$|_  $$_/   |__  $$__/| $$  | $$|_  $$_/' -ForegroundColor Magenta
    Write-Host '| $$  \ $$| $$$$| $$  | $$        | $$   | $$  | $$  | $$  ' -ForegroundColor Cyan
    Write-Host '| $$$$$$$$| $$ $$ $$  | $$ /$$$$$$| $$   | $$  | $$  | $$  ' -ForegroundColor Magenta
    Write-Host '| $$__  $$| $$  $$$$  | $$|______/| $$   | $$  | $$  | $$  ' -ForegroundColor Cyan
    Write-Host '| $$  | $$| $$\  $$$  | $$        | $$   | $$  | $$  | $$  ' -ForegroundColor Magenta
    Write-Host '| $$  | $$| $$ \  $$ /$$$$$$      | $$   |  $$$$$$/ /$$$$$$' -ForegroundColor Cyan
    Write-Host '|__/  |__/|__/  \__/|______/      |__/    \______/ |______/' -ForegroundColor Magenta
    Write-Host ''
    
    # Subtitle
    Write-Host 'ani-tui 3.3 - Terminal UI for Anime Streaming' -ForegroundColor DarkGray
    Write-Host ''
    Write-Host 'Complete Installer for Windows' -ForegroundColor Cyan
    Write-Host ''
}

# Show ASCII banner
Show-AsciiBanner

Write-Status "========================================" "Cyan"
Write-Status "Starting Installation..." "Cyan"
Write-Status "========================================" "Cyan"
Write-Status ""

# Step 0: Terminal image rendering guidance
Write-Status "Step 0: Checking terminal image rendering guidance..." "Yellow"

function Get-WindowsTerminalVersion {
    try {
        $wtPackage = Get-AppxPackage -Name "Microsoft.WindowsTerminal" -ErrorAction SilentlyContinue
        if ($wtPackage) { return $wtPackage.Version }
        
        $wtPath = (Get-Command wt -ErrorAction SilentlyContinue).Source
        if ($wtPath) {
            $versionInfo = (Get-Item $wtPath).VersionInfo
            return "$($versionInfo.FileMajorPart).$($versionInfo.FileMinorPart)"
        }
        return $null
    } catch { return $null }
}

$wtVersion = Get-WindowsTerminalVersion
if ($wtVersion) {
    Write-Status "Windows Terminal version: $wtVersion" "Gray"
    Write-Status "ani-tui will use stable halfblock previews in Windows Terminal." "Gray"
    Write-Status "For normal terminal images, Kitty or WezTerm are recommended." "Cyan"
} else {
    Write-Status "Could not detect Windows Terminal. Kitty or WezTerm are recommended for terminal images." "Yellow"
}

Write-Status ""

# Step 1: Check/Install Visual C++ Redistributable
Write-Status "Step 1: Checking Visual C++ Redistributable..." "Yellow"

$vcInstalled = $false
if (Test-Path "$env:SystemRoot\System32\vcruntime140.dll") {
    Write-Status "[OK] Visual C++ Redistributable appears to be installed" "Green"
    $vcInstalled = $true
}

if (-not $vcInstalled) {
    Write-Status "Visual C++ Redistributable not detected. Installing via winget..." "Gray"
    try {
        winget install Microsoft.VCRedist.2015+.x64 --accept-source-agreements --accept-package-agreements
        Write-Status "[OK] Visual C++ Redistributable installed" "Green"
    } catch {
        Write-Status "⚠ Could not auto-install Visual C++ Redistributable. Please install manually." "Yellow"
    }
}

# Step 2: Create installation directory
Write-Status ""
Write-Status "Step 2: Creating installation directory..." "Yellow"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Write-Status "[OK] Directory created: $InstallDir" "Green"

# Step 3: Install mpv
Write-Status ""
Write-Status "Step 3: Installing mpv (REQUIRED for video playback)..." "Yellow"
if (Test-Command "mpv") {
    Write-Status "[OK] mpv already installed" "Green"
} else {
    Write-Status "Attempting to install mpv via winget..." "Gray"
    try {
        winget install mpv --accept-source-agreements --accept-package-agreements
        Write-Status "[OK] mpv installed via winget" "Green"
    } catch {
        Write-Status "✗ Could not install mpv automatically. Please install manually." "Red"
    }
}

# Step 4: Install chafa
Write-Status ""
Write-Status "Step 4: Installing chafa (for image previews)..." "Yellow"
if (Test-Command "chafa") {
    Write-Status "[OK] chafa already installed" "Green"
} else {
    Write-Status "Attempting to install chafa via winget..." "Gray"
    try {
        winget install hpjansson.chafa --accept-source-agreements --accept-package-agreements
        Write-Status "[OK] chafa installed" "Green"
    } catch {
        Write-Status "⚠ Could not install chafa automatically." "Yellow"
    }
}

# Step 5: Install ani-tui
Write-Status ""
Write-Status "Step 5: Installing ani-tui..." "Yellow"

$response = Read-Host "Do you want to download and install ani-tui now? (Y/n)"
if ($response -eq '' -or $response -match '^[yY]') {
    Write-Status "Downloading ani-tui..." "Cyan"
    try {
        $releaseUrl = "https://github.com/silent9669/ani-tui/releases/latest/download/ani-tui-windows-x86_64.zip"
        $zipPath = "$env:TEMP\ani-tui-install.zip"
        
        # Ensure progress bar is shown by un-suppressing progress preference temporarily
        $ProgressPreference = 'Continue'
        Invoke-WebRequest -Uri $releaseUrl -OutFile $zipPath -UseBasicParsing -ErrorAction Stop
        
        Write-Status "[OK] Download complete" "Green"
        
        Write-Status "Extracting ani-tui..." "Gray"
        Expand-Archive -Path $zipPath -DestinationPath $InstallDir -Force
        Remove-Item $zipPath -Force -ErrorAction SilentlyContinue
        Write-Status "[OK] Extracted ani-tui" "Green"
    } catch {
        Write-Status "[X] Failed to download ani-tui: $_" "Red"
        exit 1
    }
} else {
    Write-Status "Skipping ani-tui download." "Yellow"
}

$binaryPath = Join-Path $InstallDir "ani-tui.exe"
if (-not (Test-Path $binaryPath)) {
    Write-Status ""
    Write-Status "[X] Installation aborted: ani-tui.exe not found in $InstallDir" "Red"
    Write-Status ""
    Read-Host "Press Enter to exit"
    exit 1
}

# Step 6: Set up PATH
Write-Status ""
Write-Status "Step 6: Setting up PATH..." "Yellow"

$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$InstallDir", "User")
    Write-Status "[OK] Added to User PATH" "Green"
} else {
    Write-Status "[OK] Already in PATH" "Green"
}
$env:PATH = "$env:PATH;$InstallDir"

# Step 7: Create wrapper script
Write-Status ""
Write-Status "Step 7: Creating shortcuts..." "Yellow"
$wrapperPath = Join-Path $InstallDir "ani-tui.cmd"
$wrapperContent = "@echo off`n`"%~dp0ani-tui.exe`" %*"
Set-Content -Path $wrapperPath -Value $wrapperContent -Force
Write-Status "[OK] Created ani-tui.cmd" "Green"

# Summary
Write-Status ""
Write-Status "========================================" "Green"
Write-Status "Installation Complete!" "Green"
Write-Status "========================================" "Green"
Write-Status "Installation Directory: $InstallDir" "White"
Write-Status ""
Write-Status "You can now run ani-tui by opening a new terminal and typing 'ani-tui'." "Cyan"
Write-Status ""
Read-Host "Press Enter to exit"
