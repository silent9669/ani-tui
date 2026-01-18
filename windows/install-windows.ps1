<#
.SYNOPSIS
    Install script for ani-tui on Windows

.DESCRIPTION
    Installs ani-tui.ps1 to a user-accessible location and optionally
    creates a command shim for easy access from any terminal.

.EXAMPLE
    .\install-windows.ps1
    Run the installer
#>

[CmdletBinding()]
param(
    [string]$InstallDir = "",
    [switch]$NoShim,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] " -ForegroundColor Blue -NoNewline
    Write-Host $Message
}

function Write-Success {
    param([string]$Message)
    Write-Host "[OK] " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[WARN] " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Show-Help {
    @"

ani-tui Windows Installer

USAGE:
    .\install-windows.ps1 [options]

OPTIONS:
    -InstallDir <path>    Custom installation directory
                          Default: $env:USERPROFILE\.ani-tui\bin
    -NoShim               Don't create ani-tui.cmd shim
    -Help                 Show this help message

EXAMPLES:
    .\install-windows.ps1
    .\install-windows.ps1 -InstallDir "C:\Tools"

"@
}

function Main {
    if ($Help) {
        Show-Help
        return
    }
    
    Write-Host ""
    Write-Host "===========================================" -ForegroundColor Cyan
    Write-Host "       ani-tui Windows Installer           " -ForegroundColor Cyan
    Write-Host "===========================================" -ForegroundColor Cyan
    Write-Host ""
    
    # Determine install directory
    if (-not $InstallDir) {
        $InstallDir = Join-Path $env:USERPROFILE ".ani-tui\bin"
    }
    
    # Get script directory
    $ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    $SourceScript = Join-Path $ScriptDir "ani-tui.ps1"
    
    # Check if source exists
    if (-not (Test-Path $SourceScript)) {
        Write-Error "Cannot find ani-tui.ps1 in $ScriptDir"
        return
    }
    
    # Create install directory
    Write-Info "Installing to: $InstallDir"
    
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        Write-Success "Created directory: $InstallDir"
    }
    
    # Copy script
    $DestScript = Join-Path $InstallDir "ani-tui.ps1"
    Copy-Item -Path $SourceScript -Destination $DestScript -Force
    Write-Success "Copied ani-tui.ps1"
    
    # Create shim
    if (-not $NoShim) {
        $ShimPath = Join-Path $InstallDir "ani-tui.cmd"
        $ShimContent = @"
@echo off
powershell -ExecutionPolicy Bypass -NoLogo -File "%~dp0ani-tui.ps1" %*
"@
        $ShimContent | Out-File -FilePath $ShimPath -Encoding ASCII
        Write-Success "Created ani-tui.cmd shim"
    }
    
    # Check if install dir is in PATH
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    $inPath = $currentPath -split ";" | Where-Object { $_ -eq $InstallDir }
    
    if (-not $inPath) {
        Write-Warn "Install directory is not in PATH"
        Write-Host ""
        Write-Host "To add to PATH, run this command in PowerShell (as Administrator):" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "  [Environment]::SetEnvironmentVariable('PATH', `$env:PATH + ';$InstallDir', 'User')" -ForegroundColor White
        Write-Host ""
        Write-Host "Or manually add this to your PATH:" -ForegroundColor Yellow
        Write-Host "  $InstallDir" -ForegroundColor White
        Write-Host ""
    }
    else {
        Write-Success "Install directory is already in PATH"
    }
    
    Write-Host ""
    Write-Host "===========================================" -ForegroundColor Green
    Write-Host "         Installation Complete!            " -ForegroundColor Green
    Write-Host "===========================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor Cyan
    Write-Host "  ani-tui                     # Interactive search"
    Write-Host "  ani-tui search `"naruto`"    # Search for anime"
    Write-Host "  ani-tui dashboard           # View watched anime"
    Write-Host ""
    Write-Host "Or run directly:" -ForegroundColor Cyan
    Write-Host "  powershell -File `"$DestScript`""
    Write-Host ""
}

# Run main
Main
