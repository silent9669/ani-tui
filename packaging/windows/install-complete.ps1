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

function Show-Step {
    param(
        [int]$Percent,
        [string]$Status
    )
    Write-Progress -Activity "ani-tui installer" -Status $Status -PercentComplete $Percent
    Write-Status ("[{0,3}%] {1}" -f $Percent, $Status) "Cyan"
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
    Write-Host 'v3.8.3' -ForegroundColor DarkGray
    Write-Host ''
}

# Show ASCII banner
Show-AsciiBanner

Show-Step 5 "Starting installation"

# Step 0: Terminal image rendering guidance
Show-Step 10 "Checking terminal image rendering guidance"

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
Show-Step 20 "Checking Visual C++ Redistributable"

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
Show-Step 35 "Creating installation directory"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Write-Status "[OK] Directory created: $InstallDir" "Green"

# Step 3: Install mpv
Write-Status ""
Show-Step 45 "Checking mpv"
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
Show-Step 55 "Checking chafa"
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
Show-Step 65 "Preparing ani-tui download"

$response = Read-Host "Do you want to download and install ani-tui now? (Y/n)"
if ($response -eq '' -or $response -match '^[yY]') {
    try {
        $releaseUrl = "https://github.com/silent9669/ani-tui/releases/latest/download/ani-tui-windows-x86_64.zip"
        $zipPath = "$env:TEMP\ani-tui-install.zip"
        
        Show-Step 70 "Downloading ani-tui"
        $ProgressPreference = 'Continue'
        Invoke-WebRequest -Uri $releaseUrl -OutFile $zipPath -UseBasicParsing -ErrorAction Stop
        
        Write-Status "[OK] Download complete" "Green"
        
        Show-Step 82 "Extracting ani-tui"
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
Show-Step 90 "Setting up PATH"

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
Show-Step 96 "Creating shortcuts"
$wrapperPath = Join-Path $InstallDir "ani-tui.cmd"
$wrapperContent = "@echo off`n`"%~dp0ani-tui.exe`" %*"
Set-Content -Path $wrapperPath -Value $wrapperContent -Force
Write-Status "[OK] Created ani-tui.cmd" "Green"

# Summary
Write-Status ""
Write-Progress -Activity "ani-tui installer" -Completed
Show-Step 100 "Installation complete"
Write-Status "========================================" "Green"
Write-Status "Installation Complete!" "Green"
Write-Status "========================================" "Green"
Write-Status "Installation Directory: $InstallDir" "White"
Write-Status ""
Write-Status "You can now run ani-tui by opening a new terminal and typing 'ani-tui'." "Cyan"
Write-Status ""
Read-Host "Press Enter to exit"
