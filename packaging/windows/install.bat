@echo off
setlocal EnableDelayedExpansion

echo ============================================
echo  Installing ani-tui - Anime Streaming TUI
echo ============================================
echo.

REM Check if running as administrator
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo Warning: Not running as administrator.
    echo Installing to user directory instead...
    set "INSTALL_DIR=%USERPROFILE%\ani-tui"
) else (
    set "INSTALL_DIR=C:\Program Files\ani-tui"
)

echo Installing to: %INSTALL_DIR%
echo.

REM Create install directory
if not exist "%INSTALL_DIR%" (
    mkdir "%INSTALL_DIR%"
)

REM Download ani-tui binary
echo Downloading ani-tui...
powershell -Command "Invoke-WebRequest -Uri 'https://github.com/silent9669/ani-tui/releases/latest/download/ani-tui-windows-x86_64.zip' -OutFile '%TEMP%\ani-tui.zip'"

if %errorLevel% neq 0 (
    echo Error: Failed to download ani-tui
    pause
    exit /b 1
)

REM Extract
echo Extracting...
powershell -Command "Expand-Archive -Path '%TEMP%\ani-tui.zip' -DestinationPath '%INSTALL_DIR%' -Force"

if %errorLevel% neq 0 (
    echo Error: Failed to extract files
    pause
    exit /b 1
)

REM Add to PATH
echo Adding to PATH...
if "%INSTALL_DIR%"=="%USERPROFILE%\ani-tui" (
    REM User PATH
    powershell -Command "[Environment]::SetEnvironmentVariable('PATH', [Environment]::GetEnvironmentVariable('PATH', 'User') + ';%INSTALL_DIR%', 'User')"
) else (
    REM System PATH
    setx /M PATH "%PATH%;%INSTALL_DIR%"
)

REM Clean up
del /f "%TEMP%\ani-tui.zip" 2>nul

echo.
echo ============================================
echo  Installation Complete!
echo ============================================
echo.
echo ani-tui has been installed to: %INSTALL_DIR%
echo.
echo Usage:
echo   ani-tui              - Start the app
echo   ani-tui -q "naruto"  - Search immediately
echo.
echo Note: You may need to restart your terminal for PATH changes to take effect.
echo.
pause