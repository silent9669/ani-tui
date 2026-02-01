# ani-tui Diagnostic Tool for Windows
# Run this if ani-tui command is not found

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "ani-tui Diagnostic Tool" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if ani-tui is in PATH
Write-Host "Checking PATH..." -ForegroundColor Yellow
$aniTuiCmd = Get-Command ani-tui -ErrorAction SilentlyContinue
if ($aniTuiCmd) {
    Write-Host "✓ ani-tui found in PATH at: $($aniTuiCmd.Source)" -ForegroundColor Green
} else {
    Write-Host "✗ ani-tui NOT found in PATH" -ForegroundColor Red
    Write-Host ""
    Write-Host "Possible solutions:" -ForegroundColor Cyan
    Write-Host "  1. Open a NEW terminal window (PATH changes require fresh session)"
    Write-Host "  2. Run the full path instead:"
    Write-Host "     %LOCALAPPDATA%\ani-tui\ani-tui.exe"
    Write-Host "  3. Reinstall using the installer"
}

# Check installation directory
Write-Host ""
Write-Host "Checking installation..." -ForegroundColor Yellow
$installDir = "$env:LOCALAPPDATA\ani-tui"
$binaryPath = Join-Path $installDir "ani-tui.exe"

if (Test-Path $binaryPath) {
    Write-Host "✓ Binary found at: $binaryPath" -ForegroundColor Green
    
    # Try to get version
    try {
        $version = & $binaryPath --version 2>&1
        Write-Host "✓ Binary is executable. Version: $version" -ForegroundColor Green
    } catch {
        Write-Host "✗ Binary exists but cannot run. Error: $_" -ForegroundColor Red
    }
} else {
    Write-Host "✗ Binary NOT found at: $binaryPath" -ForegroundColor Red
    Write-Host "  ani-tui may not be installed correctly." -ForegroundColor Yellow
}

# Check dependencies
Write-Host ""
Write-Host "Checking dependencies..." -ForegroundColor Yellow

$mpv = Get-Command mpv -ErrorAction SilentlyContinue
if ($mpv) {
    Write-Host "✓ mpv found at: $($mpv.Source)" -ForegroundColor Green
} else {
    Write-Host "✗ mpv NOT found" -ForegroundColor Red
    Write-Host "  Video playback will NOT work without mpv!" -ForegroundColor Yellow
    Write-Host "  Download from: https://mpv.io/installation/" -ForegroundColor Cyan
}

$chafa = Get-Command chafa -ErrorAction SilentlyContinue
if ($chafa) {
    Write-Host "✓ chafa found (image previews will work)" -ForegroundColor Green
} else {
    Write-Host "⚠ chafa NOT found (image previews will not work)" -ForegroundColor Yellow
}

# Environment info
Write-Host ""
Write-Host "Environment Info:" -ForegroundColor Yellow
Write-Host "  PowerShell Version: $($PSVersionTable.PSVersion)"
Write-Host "  User: $env:USERNAME"
Write-Host "  Install Path: $installDir"

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Quick Fixes:" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "If ani-tui command not found, try:" -ForegroundColor White
Write-Host "  1. Open a NEW terminal window" -ForegroundColor Yellow
Write-Host "  2. Run: \`$env:LOCALAPPDATA\ani-tui\ani-tui.exe\`" -ForegroundColor Yellow
Write-Host ""
Write-Host "If video doesn't play:" -ForegroundColor White
Write-Host "  Install mpv from: https://mpv.io/installation/" -ForegroundColor Yellow
Write-Host ""

Read-Host "Press Enter to exit"