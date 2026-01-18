@echo off
REM ani-tui v6.4 for Windows - Launcher
REM Calls the PowerShell core script directly

set "SCRIPT_DIR=%~dp0"
powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%ani-tui-core.ps1" %*
