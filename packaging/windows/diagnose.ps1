# ani-tui Windows diagnostic tool

[CmdletBinding()]
param(
    [string]$InstallDir = "$env:LOCALAPPDATA\ani-tui"
)

$ErrorActionPreference = "Continue"

function Write-Status {
    param(
        [string]$Message,
        [string]$Color = "White"
    )
    Write-Host $Message -ForegroundColor $Color
}

function Test-VersionCommand {
    param([string]$Path)

    if (-not $Path) {
        return $false
    }

    try {
        $output = & $Path --version 2>&1
        if ($LASTEXITCODE -ne 0) {
            Write-Status "  [FAIL] $Path exited with $LASTEXITCODE" "Red"
            return $false
        }
        Write-Status "  [OK] $Path" "Green"
        if ($output) {
            Write-Status "       $($output | Select-Object -First 1)" "DarkGray"
        }
        return $true
    } catch {
        Write-Status "  [FAIL] $Path - $_" "Red"
        return $false
    }
}

Write-Status "========================================" "Cyan"
Write-Status "ani-tui Windows diagnostics" "Cyan"
Write-Status "========================================" "Cyan"
Write-Status "Install directory: $InstallDir"
Write-Status ""

$binaryPath = Join-Path $InstallDir "ani-tui.exe"
Write-Status "ani-tui binary:" "Yellow"
if (Test-Path $binaryPath) {
    Test-VersionCommand $binaryPath | Out-Null
} else {
    Write-Status "  [FAIL] Missing: $binaryPath" "Red"
}

Write-Status ""
Write-Status "PATH command:" "Yellow"
$aniCommand = Get-Command ani-tui -ErrorAction SilentlyContinue
if ($aniCommand) {
    Write-Status "  [OK] ani-tui resolves to: $($aniCommand.Source)" "Green"
} else {
    Write-Status "  [WARN] ani-tui is not visible in this terminal PATH" "Yellow"
    Write-Status "         Open a new terminal or run: $binaryPath" "DarkGray"
}

Write-Status ""
Write-Status "mpv candidates:" "Yellow"
$mpvCandidates = New-Object System.Collections.Generic.List[string]
if ($env:ANI_TUI_PLAYER) {
    $mpvCandidates.Add($env:ANI_TUI_PLAYER)
    Write-Status "  ANI_TUI_PLAYER: $env:ANI_TUI_PLAYER" "DarkGray"
} else {
    Write-Status "  ANI_TUI_PLAYER is not set in this terminal" "Yellow"
}

$mpvCommand = Get-Command mpv -ErrorAction SilentlyContinue
if ($mpvCommand) {
    $mpvCandidates.Add($mpvCommand.Source)
}
$mpvCandidates.Add((Join-Path $InstallDir "mpv.exe"))
$mpvCandidates.Add((Join-Path $InstallDir "tools\mpv\mpv.exe"))

$foundMpv = $false
foreach ($candidate in ($mpvCandidates | Select-Object -Unique)) {
    if (Test-Path $candidate) {
        if (Test-VersionCommand $candidate) {
            $foundMpv = $true
        }
    } else {
        Write-Status "  [MISS] $candidate" "DarkGray"
    }
}

if (-not $foundMpv) {
    Write-Status "  [FAIL] No usable mpv.exe found" "Red"
    Write-Status "         Rerun the installer or set ANI_TUI_PLAYER to the full mpv.exe path." "Yellow"
}

Write-Status ""
Write-Status "Recent mpv log:" "Yellow"
$mpvLog = Join-Path $env:TEMP "ani-tui-mpv.log"
if (Test-Path $mpvLog) {
    Write-Status "  $mpvLog" "DarkGray"
    Get-Content $mpvLog -Tail 40
} else {
    Write-Status "  [MISS] No mpv log found at $mpvLog" "DarkGray"
}

Write-Status ""
Write-Status "Environment:" "Yellow"
Write-Status "  PowerShell: $($PSVersionTable.PSVersion)"
Write-Status "  Windows:    $([Environment]::OSVersion.VersionString)"
Write-Status "  TEMP:       $env:TEMP"

Write-Status ""
Write-Status "Recommended reinstall command:" "Cyan"
Write-Status '  powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://github.com/silent9669/ani-tui/releases/latest/download/install.ps1 -OutFile install.ps1; .\install.ps1"'
