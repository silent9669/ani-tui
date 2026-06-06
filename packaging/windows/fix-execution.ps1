# Compatibility wrapper for the old Windows fix tool.
# The supported troubleshooting entrypoint is diagnose.ps1.

$ErrorActionPreference = "Stop"

$localDiagnostic = Join-Path $PSScriptRoot "diagnose.ps1"
if (Test-Path $localDiagnostic) {
    & $localDiagnostic
    exit
}

$remoteDiagnostic = "https://raw.githubusercontent.com/silent9669/ani-tui/master/packaging/windows/diagnose.ps1"
$tempDiagnostic = Join-Path $env:TEMP "ani-tui-diagnose.ps1"
Invoke-WebRequest -Uri $remoteDiagnostic -OutFile $tempDiagnostic -UseBasicParsing -ErrorAction Stop
& $tempDiagnostic
