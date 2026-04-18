# ani-tui Smart Installer for Windows
# Usage: iwr -useb https://raw.githubusercontent.com/silent9669/ani-tui/main/packaging/windows/install.ps1 | iex

$ErrorActionPreference = "Stop"

Clear-Host
Write-Host "🎬 Welcome to ani-tui Smart Installer" -ForegroundColor Cyan
Write-Host "--------------------------------------"

$choice = Read-Host "Do you want to install ani-tui and dependencies (mpv, scoop)? [Y/N]"
if ($choice -ne "Y" -and $choice -ne "y") {
    Write-Host "❌ Installation cancelled." -ForegroundColor Red
    exit
}

function Show-Progress($activity, $percent) {
    Write-Progress -Activity $activity -Status "$percent% Complete" -PercentComplete $percent
}

# 1. Check for Scoop
Show-Progress "Checking for Scoop..." 10
$scoop = Get-Command scoop -ErrorAction SilentlyContinue
if (-not $scoop) {
    Show-Progress "Installing Scoop package manager..." 20
    Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
    iwr -useb https://get.scoop.sh | iex
}

# 2. Install Dependencies
Show-Progress "Installing dependencies (mpv, chafa)..." 40
scoop install mpv chafa -s

# 3. Download ani-tui
Show-Progress "Fetching latest release..." 60
$repo = "silent9669/ani-tui"
$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest"
$version = $release.tag_name
$arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }
$asset = $release.assets | Where-Object { $_.name -like "*windows-$arch*" }

$installDir = "$env:LOCALAPPDATA\ani-tui"
if (-not (Test-Path $installDir)) { New-Item -ItemType Directory -Path $installDir -Force | Out-Null }

$binPath = Join-Path $installDir "ani-tui.exe"
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $binPath

# 4. Add to PATH
Show-Progress "Configuring environment..." 80
$path = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($path -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$path;$installDir", "User")
    $env:PATH += ";$installDir"
}

Show-Progress "Finalizing..." 100
Write-Progress -Activity "Installation Complete" -Completed

Write-Host "`n✅ ani-tui $version installed successfully!" -ForegroundColor Green
Write-Host "🚀 Close and restart your terminal, then type 'ani-tui' to start." -ForegroundColor Yellow
