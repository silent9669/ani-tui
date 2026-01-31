# Windows Installation Script for ani-tui
# Usage: iwr -useb https://raw.githubusercontent.com/silent9669/ani-tui/main/packaging/windows/install.ps1 | iex

$ErrorActionPreference = "Stop"

Write-Host "🎬 Installing ani-tui..." -ForegroundColor Cyan

# Check if running as administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")
if (-not $isAdmin) {
    Write-Host "⚠️  Please run PowerShell as Administrator for system-wide installation" -ForegroundColor Yellow
    Write-Host "   Or use: iwr -useb <url> | iex" -ForegroundColor Gray
}

# Check for scoop
$scoopInstalled = Get-Command scoop -ErrorAction SilentlyContinue
if (-not $scoopInstalled) {
    Write-Host "📦 Installing Scoop package manager..." -ForegroundColor Yellow
    Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
    Invoke-RestMethod -Uri https://get.scoop.sh | Invoke-Expression
}

# Install dependencies
Write-Host "📦 Checking dependencies..." -ForegroundColor Cyan

$chafaInstalled = Get-Command chafa -ErrorAction SilentlyContinue
if (-not $chafaInstalled) {
    Write-Host "  Installing chafa..." -ForegroundColor Gray
    scoop install chafa
}

$mpvInstalled = Get-Command mpv -ErrorAction SilentlyContinue
if (-not $mpvInstalled) {
    Write-Host "  Installing mpv..." -ForegroundColor Gray
    scoop install mpv
}

# Get latest release
$repo = "silent9669/ani-tui"
Write-Host "⬇️  Fetching latest release..." -ForegroundColor Cyan

$releases = Invoke-RestMethod -Uri "https://api.github.com/repos/${repo}/releases/latest"
$latestVersion = $releases.tag_name

Write-Host "   Latest version: $latestVersion" -ForegroundColor Gray

# Determine architecture
$arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }
$binaryName = "ani-tui-windows-${arch}.exe"
$downloadUrl = $releases.assets | Where-Object { $_.name -eq $binaryName } | Select-Object -ExpandProperty browser_download_url

if (-not $downloadUrl) {
    # Fallback to building from source
    Write-Host "⚠️  Pre-built binary not found. Building from source..." -ForegroundColor Yellow
    
    # Check for Rust
    $rustInstalled = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $rustInstalled) {
        Write-Host "❌ Rust is not installed. Please install Rust: https://rustup.rs/" -ForegroundColor Red
        exit 1
    }
    
    # Clone and build
    $tempDir = Join-Path $env:TEMP "ani-tui-build"
    if (Test-Path $tempDir) {
        Remove-Item -Recurse -Force $tempDir
    }
    
    git clone "https://github.com/${repo}.git" $tempDir
    Set-Location $tempDir
    cargo build --release
    
    $sourcePath = Join-Path $tempDir "target\release\ani-tui.exe"
} else {
    # Download pre-built binary
    $tempDir = Join-Path $env:TEMP "ani-tui-install"
    if (Test-Path $tempDir) {
        Remove-Item -Recurse -Force $tempDir
    }
    New-Item -ItemType Directory -Path $tempDir | Out-Null
    
    $sourcePath = Join-Path $tempDir "ani-tui.exe"
    Write-Host "⬇️  Downloading..." -ForegroundColor Gray
    Invoke-WebRequest -Uri $downloadUrl -OutFile $sourcePath
}

# Install to scoop apps directory or Program Files
$installDir = if (Test-Path "$env:USERPROFILE\scoop\apps") {
    Join-Path $env:USERPROFILE "scoop\apps\ani-tui\current"
} else {
    "$env:ProgramFiles\ani-tui"
}

if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

Write-Host "🔧 Installing to $installDir..." -ForegroundColor Cyan
Copy-Item -Path $sourcePath -Destination (Join-Path $installDir "ani-tui.exe") -Force

# Add to PATH if not already there
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$installDir*") {
    Write-Host "📝 Adding to PATH..." -ForegroundColor Gray
    [Environment]::SetEnvironmentVariable("PATH", "$userPath;$installDir", "User")
    $env:PATH = "$env:PATH;$installDir"
}

# Cleanup
if (Test-Path $tempDir) {
    Remove-Item -Recurse -Force $tempDir
}

# Verify installation
$aniTuiInstalled = Get-Command ani-tui -ErrorAction SilentlyContinue
if ($aniTuiInstalled) {
    Write-Host "✅ ani-tui installed successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "🚀 Usage:" -ForegroundColor Cyan
    Write-Host "   ani-tui              # Start the app" -ForegroundColor White
    Write-Host "   ani-tui -q 'naruto'  # Search immediately" -ForegroundColor White
    Write-Host ""
    Write-Host "📖 Run 'ani-tui --help' for more options" -ForegroundColor Gray
} else {
    Write-Host "❌ Installation failed. Please check your PATH." -ForegroundColor Red
    exit 1
}