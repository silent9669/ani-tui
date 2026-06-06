@echo off
setlocal

echo ani-tui Windows installer
echo Redirecting to the supported PowerShell installer...
echo.

set "INSTALLER=%TEMP%\ani-tui-install.ps1"
powershell -NoProfile -ExecutionPolicy Bypass -Command "Invoke-WebRequest -Uri 'https://github.com/silent9669/ani-tui/releases/latest/download/install.ps1' -OutFile '%INSTALLER%' -UseBasicParsing; & '%INSTALLER%'"
set "EXIT_CODE=%ERRORLEVEL%"

if not "%EXIT_CODE%"=="0" (
    echo.
    echo Installer failed with exit code %EXIT_CODE%.
    pause
)

exit /b %EXIT_CODE%
