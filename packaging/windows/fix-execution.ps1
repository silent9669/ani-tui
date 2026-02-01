# Windows Diagnostic and Fix Tool for ani-tui
# Run this to diagnose and fix ani-tui issues

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "ani-tui Diagnostic Tool" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$installDir = "$env:LOCALAPPDATA\ani-tui"
$binaryPath = Join-Path $installDir "ani-tui.exe"

# Test 1: Check if binary exists
Write-Host "Test 1: Checking if ani-tui.exe exists..." -ForegroundColor Yellow
if (Test-Path $binaryPath) {
    Write-Host "  ✓ Found at: $binaryPath" -ForegroundColor Green
    
    # Check file size
    $fileInfo = Get-Item $binaryPath
    Write-Host "  File size: $($fileInfo.Length) bytes" -ForegroundColor Gray
} else {
    Write-Host "  ✗ NOT FOUND!" -ForegroundColor Red
    exit 1
}

# Test 2: Try to run with version check
Write-Host ""
Write-Host "Test 2: Testing ani-tui execution..." -ForegroundColor Yellow
try {
    # Run with redirect to capture any output
    $process = Start-Process -FilePath $binaryPath -ArgumentList "--version" -PassThru -Wait -WindowStyle Hidden
    $exitCode = $process.ExitCode
    Write-Host "  Exit code: $exitCode" -ForegroundColor Gray
    
    if ($exitCode -eq 0) {
        Write-Host "  ✓ ani-tui runs successfully" -ForegroundColor Green
    } else {
        Write-Host "  ✗ ani-tui exited with error code: $exitCode" -ForegroundColor Red
    }
} catch {
    Write-Host "  ✗ Error running ani-tui: $_" -ForegroundColor Red
}

# Test 3: Check dependencies
Write-Host ""
Write-Host "Test 3: Checking dependencies..." -ForegroundColor Yellow

# Check for required DLLs on Windows
$requiredDlls = @("vcruntime140.dll", "msvcp140.dll")
foreach ($dll in $requiredDlls) {
    $dllPath = Join-Path $installDir $dll
    if (Test-Path $dllPath) {
        Write-Host "  ✓ $dll found" -ForegroundColor Green
    } else {
        Write-Host "  ⚠ $dll not found (may be in system PATH)" -ForegroundColor Yellow
    }
}

# Test 4: Try running with full path
Write-Host ""
Write-Host "Test 4: Testing direct execution..." -ForegroundColor Yellow
Write-Host "  Running: $binaryPath --help" -ForegroundColor Gray
try {
    $output = & $binaryPath --help 2>&1
    if ($output) {
        Write-Host "  ✓ Got output:" -ForegroundColor Green
        Write-Host "  $output" -ForegroundColor Gray
    } else {
        Write-Host "  ⚠ No output captured" -ForegroundColor Yellow
    }
} catch {
    Write-Host "  ✗ Error: $_" -ForegroundColor Red
}

# Test 5: Check PATH
Write-Host ""
Write-Host "Test 5: Checking PATH..." -ForegroundColor Yellow
$pathDirs = $env:PATH -split ';'
$aniTuiInPath = $false
foreach ($dir in $pathDirs) {
    if ($dir -like "*ani-tui*") {
        Write-Host "  Found in PATH: $dir" -ForegroundColor Green
        $aniTuiInPath = $true
    }
}

if (-not $aniTuiInPath) {
    Write-Host "  ✗ ani-tui directory NOT in PATH!" -ForegroundColor Red
}

# Solutions
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Solutions:" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "If ani-tui command does nothing:" -ForegroundColor White
Write-Host "  1. Try running directly:" -ForegroundColor Yellow
Write-Host "     $binaryPath" -ForegroundColor Cyan
Write-Host ""
Write-Host "  2. Check if you have Visual C++ Redistributable installed:" -ForegroundColor Yellow
Write-Host "     Download from: https://aka.ms/vs/17/release/vc_redist.x64.exe" -ForegroundColor Cyan
Write-Host ""
Write-Host "  3. Try installing with dependencies:" -ForegroundColor Yellow
Write-Host "     Download Visual C++ Redistributable first, then reinstall ani-tui" -ForegroundColor Cyan
Write-Host ""
Write-Host "  4. Alternative: Use the full path always:" -ForegroundColor Yellow
Write-Host "     Set-Alias ani-tui '$binaryPath'" -ForegroundColor Cyan
Write-Host "     Add this to your PowerShell profile" -ForegroundColor Cyan

Read-Host "`nPress Enter to exit"