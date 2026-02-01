# Complete Windows Installer for ani-tui
# This script installs ani-tui with ALL dependencies automatically
# No manual setup required!

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

Write-Status "========================================" "Cyan"
Write-Status "ani-tui Complete Installer" "Cyan"
Write-Status "========================================" "Cyan"
Write-Status ""

# Step 1: Create installation directory
Write-Status "Step 1: Creating installation directory..." "Yellow"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Write-Status "✓ Directory created: $InstallDir" "Green"

# Step 2: Install mpv (Required)
Write-Status ""
Write-Status "Step 2: Installing mpv (REQUIRED for video playback)..." "Yellow"

if (Test-Command "mpv") {
    Write-Status "✓ mpv already installed" "Green"
} else {
    Write-Status "Downloading mpv..." "Gray"
    try {
        # Download mpv for Windows
        $mpvUrl = "https://github.com/shinchiro/mpv-winbuild-cmake/releases/download/20241230/mpv-x86_64-windows-gnu-20241230.7z"
        $mpvTemp = "$env:TEMP\mpv.7z"
        $mpvDir = "$InstallDir\mpv"
        
        Invoke-WebRequest -Uri $mpvUrl -OutFile $mpvTemp -UseBasicParsing -ErrorAction Stop
        
        Write-Status "Extracting mpv..." "Gray"
        New-Item -ItemType Directory -Force -Path $mpvDir | Out-Null
        
        # Use Expand-Archive for 7z if available, otherwise use tar
        if (Get-Command 7z -ErrorAction SilentlyContinue) {
            & 7z x "$mpvTemp" -o"$mpvDir" -y
        } else {
            # Fallback: Download portable mpv
            $mpvPortableUrl = "https://sourceforge.net/projects/mpv-player-windows/files/64bit/mpv-x86_64-20241230-git-8.7z/download"
            Write-Status "Downloading mpv (alternative method)..." "Gray"
            Invoke-WebRequest -Uri $mpvPortableUrl -OutFile $mpvTemp -UseBasicParsing -UserAgent "Mozilla/5.0"
            
            # Extract using Windows built-in (if it's a zip)
            if ($mpvTemp -like "*.zip") {
                Expand-Archive -Path $mpvTemp -DestinationPath $mpvDir -Force
            }
        }
        
        # Add mpv to PATH
        $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
        if ($currentPath -notlike "*$mpvDir*") {
            [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$mpvDir", "User")
            $env:PATH = "$env:PATH;$mpvDir"
        }
        
        Remove-Item $mpvTemp -Force -ErrorAction SilentlyContinue
        Write-Status "✓ mpv installed successfully" "Green"
    } catch {
        Write-Status "⚠ Could not auto-install mpv. Please install manually:" "Yellow"
        Write-Status "  https://mpv.io/installation/" "Cyan"
    }
}

# Step 3: Install chafa (Optional but recommended)
Write-Status ""
Write-Status "Step 3: Installing chafa (for image previews)..." "Yellow"

if (Test-Command "chafa") {
    Write-Status "✓ chafa already installed" "Green"
} else {
    Write-Status "Downloading chafa..." "Gray"
    try {
        # Download chafa for Windows
        $chafaUrl = "https://hpjansson.org/chafa/releases/static/chafa-1.14.0-x86_64-windows.zip"
        $chafaTemp = "$env:TEMP\chafa.zip"
        $chafaDir = "$InstallDir\chafa"
        
        Invoke-WebRequest -Uri $chafaUrl -OutFile $chafaTemp -UseBasicParsing -ErrorAction Stop
        
        Write-Status "Extracting chafa..." "Gray"
        Expand-Archive -Path $chafaTemp -DestinationPath $chafaDir -Force
        
        # Add chafa to PATH
        $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
        if ($currentPath -notlike "*$chafaDir*") {
            [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$chafaDir", "User")
            $env:PATH = "$env:PATH;$chafaDir"
        }
        
        Remove-Item $chafaTemp -Force -ErrorAction SilentlyContinue
        Write-Status "✓ chafa installed successfully" "Green"
    } catch {
        Write-Status "⚠ Could not auto-install chafa (optional)" "Yellow"
    }
}

# Step 4: Install ani-tui
Write-Status ""
Write-Status "Step 4: Installing ani-tui..." "Yellow"

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

# Step 6: Create wrapper scripts for immediate use
Write-Status ""
Write-Status "Step 6: Creating shortcuts..." "Yellow"

# Create ani-tui.cmd in install directory for immediate use
$wrapperPath = Join-Path $InstallDir "ani-tui.cmd"
$wrapperContent = @'
@echo off
"%~dp0ani-tui.exe" %*
'@
Set-Content -Path $wrapperPath -Value $wrapperContent -Force
Write-Status "✓ Created ani-tui.cmd" "Green"

# Also create a .bat version
$batPath = Join-Path $InstallDir "ani-tui.bat"
Set-Content -Path $batPath -Value $wrapperContent -Force

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

# Check PATH
$aniTuiInPath = Get-Command ani-tui -ErrorAction SilentlyContinue
if ($aniTuiInPath) {
    Write-Status "✓ ani-tui command is available in PATH" "Green"
} else {
    Write-Status "⚠ ani-tui not yet in PATH (requires new terminal)" "Yellow"
}

# Check dependencies
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
Write-Status "IMPORTANT: You MUST open a NEW terminal window to use 'ani-tui' command" "Yellow"
Write-Status ""
Write-Status "After opening new terminal, you can run:" "White"
Write-Status "  ani-tui              - Start the app" "Cyan"
Write-Status "  ani-tui -q ""naruto"" - Search immediately" "Cyan"
Write-Status ""
Write-Status "Or run directly now (without new terminal):" "White"
Write-Status "  $InstallDir\ani-tui.exe" "Cyan"
Write-Status ""

Read-Host "Press Enter to exit"