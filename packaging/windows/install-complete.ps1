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
    <#
    .SYNOPSIS
    Displays the ANI-TUI ASCII art banner with alternating Cyan/Magenta colors.
    Uses single quotes to avoid $ variable interpolation in PowerShell.
    #>
    
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
    Write-Host 'Terminal UI for Anime Streaming' -ForegroundColor DarkGray
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

# Step 0: Check Windows Terminal Version
Write-Status "Step 0: Checking Windows Terminal version..." "Yellow"

function Get-WindowsTerminalVersion {
    try {
        # Try to get version from Windows Terminal package
        $wtPackage = Get-AppxPackage -Name "Microsoft.WindowsTerminal" -ErrorAction SilentlyContinue
        if ($wtPackage) {
            return $wtPackage.Version
        }
        
        # Try to get from wt.exe
        $wtPath = (Get-Command wt -ErrorAction SilentlyContinue).Source
        if ($wtPath) {
            $versionInfo = (Get-Item $wtPath).VersionInfo
            return "$($versionInfo.FileMajorPart).$($versionInfo.FileMinorPart)"
        }
        
        return $null
    } catch {
        return $null
    }
}

$wtVersion = Get-WindowsTerminalVersion

if ($wtVersion) {
    Write-Status "Windows Terminal version: $wtVersion" "Gray"
    
    # Parse version (e.g., "1.18.3181.0" -> major=1, minor=18)
    $versionParts = $wtVersion -split '\.'
    $major = [int]$versionParts[0]
    $minor = [int]$versionParts[1]
    
    # Check if version is >= 1.22
    if ($major -lt 1 -or ($major -eq 1 -and $minor -lt 22)) {
        Write-Status ""
        Write-Status "❌ Windows Terminal $wtVersion is too old" "Red"
        Write-Status ""
        Write-Status "⚠️  Windows Terminal 1.22+ is required for image previews" "Yellow"
        Write-Status ""
        Write-Status "Please upgrade Windows Terminal first:" "Yellow"
        Write-Status "  winget install --id Microsoft.WindowsTerminal -e" "Cyan"
        Write-Status ""
        Write-Status "After upgrading, please re-run this installer." "Yellow"
        Write-Status ""
        
        $continue = Read-Host "Continue without image support? (y/N)"
        if ($continue -ne 'y' -and $continue -ne 'Y') {
            exit 1
        }
        Write-Status "⚠️  Continuing without image preview support" "Yellow"
    } else {
        Write-Status "✓ Windows Terminal $wtVersion is supported" "Green"
    }
} else {
    Write-Status "⚠️  Could not detect Windows Terminal version" "Yellow"
    Write-Status "   Images may not work properly" "Yellow"
}

Write-Status ""

# Step 2: Check/Install Visual C++ Redistributable (CRITICAL)
Write-Status "Step 2: Checking Visual C++ Redistributable..." "Yellow"
Write-Status "This is REQUIRED for ani-tui to run on Windows!" "Red"

$vcInstalled = $false
try {
    # Check if VCRUNTIME140.dll exists in System32
    if (Test-Path "$env:SystemRoot\System32\vcruntime140.dll") {
        Write-Status "✓ Visual C++ Redistributable appears to be installed" "Green"
        $vcInstalled = $true
    }
} catch {}

if (-not $vcInstalled) {
    Write-Status "Visual C++ Redistributable not detected." "Yellow"
    Write-Status "Installing via winget..." "Gray"
    
    try {
        winget install Microsoft.VCRedist.2015+.x64 --accept-source-agreements --accept-package-agreements
        Write-Status "✓ Visual C++ Redistributable installed" "Green"
        Write-Status "IMPORTANT: You may need to restart your computer after installation!" "Red"
    } catch {
        Write-Status "⚠ Could not auto-install Visual C++ Redistributable" "Yellow"
        Write-Status "Please download and install manually:" "Yellow"
        Write-Status "https://aka.ms/vs/17/release/vc_redist.x64.exe" "Cyan"
        Write-Status ""
        $continue = Read-Host "Continue anyway? (y/N)"
        if ($continue -ne 'y' -and $continue -ne 'Y') {
            exit 1
        }
    }
}

# Step 2: Create installation directory
Write-Status ""
Write-Status "Step 2: Creating installation directory..." "Yellow"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Write-Status "✓ Directory created: $InstallDir" "Green"

# Step 3: Install mpv using winget (most reliable)
Write-Status ""
Write-Status "Step 3: Installing mpv (REQUIRED for video playback)..." "Yellow"

if (Test-Command "mpv") {
    Write-Status "✓ mpv already installed" "Green"
} else {
    Write-Status "Attempting to install mpv via winget..." "Gray"
    try {
        winget install mpv --accept-source-agreements --accept-package-agreements
        Write-Status "✓ mpv installed via winget" "Green"
    } catch {
        Write-Status "⚠ winget failed, trying alternative method..." "Yellow"
        
        # Fallback: Download portable mpv as zip
        try {
            $mpvUrl = "https://github.com/mpv-player/mpv/releases/download/v0.37.0/mpv-0.37.0-windows-x86_64.zip"
            $mpvTemp = "$env:TEMP\mpv.zip"
            $mpvDir = "$InstallDir\mpv"
            
            Invoke-WebRequest -Uri $mpvUrl -OutFile $mpvTemp -UseBasicParsing -ErrorAction Stop
            
            Write-Status "Extracting mpv..." "Gray"
            New-Item -ItemType Directory -Force -Path $mpvDir | Out-Null
            Expand-Archive -Path $mpvTemp -DestinationPath $mpvDir -Force
            Remove-Item $mpvTemp -Force -ErrorAction SilentlyContinue
            
            # Add mpv to PATH
            $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
            if ($currentPath -notlike "*$mpvDir*") {
                [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$mpvDir", "User")
                $env:PATH = "$env:PATH;$mpvDir"
            }
            
            Write-Status "✓ mpv installed (portable)" "Green"
        } catch {
            Write-Status "✗ Could not install mpv automatically" "Red"
            Write-Status "Please install manually from: https://mpv.io/installation/" "Yellow"
        }
    }
}

# Step 4: Install chafa (Optional)
Write-Status ""
Write-Status "Step 4: Installing chafa (for image previews)..." "Yellow"

if (Test-Command "chafa") {
    Write-Status "✓ chafa already installed" "Green"
} else {
    # Try winget first
    Write-Status "Attempting to install chafa via winget..." "Gray"
    try {
        winget install hpjansson.chafa --accept-source-agreements --accept-package-agreements
        Write-Status "✓ chafa installed via winget" "Green"
    } catch {
        # Fallback to scoop
        Write-Status "⚠ winget failed, trying scoop..." "Yellow"
        try {
            if (Test-Command "scoop") {
                scoop install chafa
                Write-Status "✓ chafa installed via scoop" "Green"
            } else {
                Write-Status "⚠ scoop not available. Install chafa manually or via winget." "Yellow"
            }
        } catch {
            Write-Status "⚠ Could not install chafa (optional)" "Yellow"
        }
    }
}

# Step 5: Install ani-tui
Write-Status ""
Write-Status "Step 5: Installing ani-tui..." "Yellow"

$existingPath = Get-Command ani-tui -ErrorAction SilentlyContinue
if ($existingPath) {
    Write-Status "ani-tui already installed at: $($existingPath.Source)" "Yellow"
    $response = Read-Host "Do you want to reinstall/upgrade? (y/N)"
    if ($response -ne 'y' -and $response -ne 'Y') {
        Write-Status "Skipping ani-tui installation." "Yellow"
    }
}

Write-Status "Downloading ani-tui..." "Gray"
try {
    $releaseUrl = "https://github.com/silent9669/ani-tui/releases/latest/download/ani-tui-windows-x86_64.zip"
    $zipPath = "$env:TEMP\ani-tui-install.zip"
    
    Invoke-WebRequest -Uri $releaseUrl -OutFile $zipPath -UseBasicParsing -ErrorAction Stop
    Write-Status "✓ Downloaded ani-tui" "Green"
    
    Write-Status "Extracting ani-tui..." "Gray"
    Expand-Archive -Path $zipPath -DestinationPath $InstallDir -Force
    Remove-Item $zipPath -Force -ErrorAction SilentlyContinue
    Write-Status "✓ Extracted ani-tui" "Green"
} catch {
    Write-Status "✗ Failed to download ani-tui: $_" "Red"
    exit 1
}

# Step 5: Add ani-tui to PATH
Write-Status ""
Write-Status "Step 5: Setting up PATH..." "Yellow"

$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$InstallDir", "User")
    Write-Status "✓ Added to User PATH" "Green"
} else {
    Write-Status "✓ Already in PATH" "Green"
}

# Update current session PATH
$env:PATH = "$env:PATH;$InstallDir"

# Step 6: Create wrapper scripts
Write-Status ""
Write-Status "Step 6: Creating shortcuts..." "Yellow"

$wrapperPath = Join-Path $InstallDir "ani-tui.cmd"
$wrapperContent = @'
@echo off
"%~dp0ani-tui.exe" %*
'@
Set-Content -Path $wrapperPath -Value $wrapperContent -Force
Write-Status "✓ Created ani-tui.cmd" "Green"

# Step 7: Test installation
Write-Status ""
Write-Status "Step 7: Testing installation..." "Yellow"

$binaryPath = Join-Path $InstallDir "ani-tui.exe"
if (Test-Path $binaryPath) {
    Write-Status "✓ Binary found at: $binaryPath" "Green"
    
    try {
        $version = & $binaryPath --version 2>&1
        if ($version) {
            Write-Status "✓ ani-tui is working! Version: $version" "Green"
        }
    } catch {
        Write-Status "⚠ Binary found but couldn't verify version" "Yellow"
    }
} else {
    Write-Status "✗ Binary not found!" "Red"
}

# Step 8: Check dependencies
Write-Status ""
Write-Status "Step 8: Checking dependencies..." "Yellow"

if (Test-Command "mpv") {
    Write-Status "✓ mpv: INSTALLED" "Green"
} else {
    Write-Status "✗ mpv: NOT FOUND - Videos will not play!" "Red"
}

if (Test-Command "chafa") {
    Write-Status "✓ chafa: INSTALLED" "Green"
} else {
    Write-Status "⚠ chafa: Not found - Images won't display" "Yellow"
}

# Installation Summary
Write-Status ""
Write-Status "========================================" "Green"
Write-Status "Installation Complete!" "Green"
Write-Status "========================================" "Green"
Write-Status ""
Write-Status "Installation Directory: $InstallDir" "White"
Write-Status ""
Write-Status "⚠⚠⚠  IMPORTANT  ⚠⚠⚠" "Red"
Write-Status "You MUST do ONE of the following:" "Yellow"
Write-Status ""
Write-Status "1. Restart your computer (recommended)" "Cyan"
Write-Status "   Then open a new terminal and run: ani-tui" "White"
Write-Status ""
Write-Status "2. Or run directly now (without restart):" "Cyan"
Write-Status "   $InstallDir\ani-tui.exe" "White"
Write-Status ""

Read-Host "Press Enter to exit"