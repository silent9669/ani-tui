<#
.SYNOPSIS
    ani-tui v6.2 - Windows Entry Point
.DESCRIPTION
    Simple wrapper that calls the core script.
#>
param(
    [Parameter(Position=0)][string]$Command = "",
    [Parameter(ValueFromRemainingArguments)][string[]]$Args
)

$script = Join-Path $PSScriptRoot "ani-tui-core.ps1"
if (Test-Path $script) {
    & $script $Command @Args
} else {
    Write-Host "Error: ani-tui-core.ps1 not found" -ForegroundColor Red
    Write-Host "Reinstall: iex (irm https://raw.githubusercontent.com/silent9669/ani-tui/master/windows/install-windows.ps1)"
}
