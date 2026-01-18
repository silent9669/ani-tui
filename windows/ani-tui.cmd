@echo off
setlocal enabledelayedexpansion
REM ani-tui for Windows - Main entry point (batch wrapper)
REM This calls the PowerShell script which handles the main logic

set "SCRIPT_DIR=%~dp0"
set "PS_SCRIPT=%SCRIPT_DIR%ani-tui-core.ps1"

REM Pass all arguments to PowerShell
powershell -NoProfile -NoLogo -ExecutionPolicy Bypass -File "%PS_SCRIPT%" %*
