@echo off
setlocal EnableDelayedExpansion

echo ============================================
echo  ani-tui Complete Windows Installer
echo ============================================
echo.
echo This will install ani-tui with all dependencies:
echo  - mpv (required for video playback)
echo  - chafa (optional for image previews)
echo  - ani-tui
echo.

set "INSTALL_DIR=%USERPROFILE%\ani-tui"

REM Create install directory
if not exist "%INSTALL_DIR%" (
    mkdir "%INSTALL_DIR%"
)

echo [1/4] Downloading mpv (REQUIRED)...
echo.

REM Check if mpv is already installed
where mpv >nul 2>&1
if %errorLevel% equ 0 (
    echo ✓ mpv already installed
) else (
    echo Downloading mpv...
    powershell -Command "Invoke-WebRequest -Uri 'https://sourceforge.net/projects/mpv-player-windows/files/64bit/mpv-x86_64-20241230-git-8.7z/download' -OutFile '%TEMP%\mpv.7z' -UseBasicParsing -UserAgent 'Mozilla/5.0'"
    
    echo Extracting mpv...
    if exist "%TEMP%\mpv.7z" (
        mkdir "%INSTALL_DIR%\mpv" 2>nul
        powershell -Command "Expand-Archive -Path '%TEMP%\mpv.7z' -DestinationPath '%INSTALL_DIR%\mpv' -Force"
        del /f "%TEMP%\mpv.7z" 2>nul
        
        REM Add mpv to PATH
        powershell -Command "[Environment]::SetEnvironmentVariable('PATH', [Environment]::GetEnvironmentVariable('PATH', 'User') + ';%INSTALL_DIR%\mpv', 'User')"
        echo ✓ mpv installed
    ) else (
        echo ⚠ Could not auto-install mpv. Please install manually:
        echo   https://mpv.io/installation/
    )
)

echo.
echo [2/4] Downloading chafa (optional)...

where chafa >nul 2>&1
if %errorLevel% equ 0 (
    echo ✓ chafa already installed
) else (
    echo Downloading chafa...
    powershell -Command "Invoke-WebRequest -Uri 'https://hpjansson.org/chafa/releases/static/chafa-1.14.0-x86_64-windows.zip' -OutFile '%TEMP%\chafa.zip' -UseBasicParsing"
    
    if exist "%TEMP%\chafa.zip" (
        mkdir "%INSTALL_DIR%\chafa" 2>nul
        powershell -Command "Expand-Archive -Path '%TEMP%\chafa.zip' -DestinationPath '%INSTALL_DIR%\chafa' -Force"
        del /f "%TEMP%\chafa.zip" 2>nul
        
        REM Add chafa to PATH
        powershell -Command "[Environment]::SetEnvironmentVariable('PATH', [Environment]::GetEnvironmentVariable('PATH', 'User') + ';%INSTALL_DIR%\chafa', 'User')"
        echo ✓ chafa installed
    ) else (
        echo ⚠ Could not auto-install chafa (optional)
    )
)

echo.
echo [3/4] Downloading ani-tui...

powershell -Command "Invoke-WebRequest -Uri 'https://github.com/silent9669/ani-tui/releases/latest/download/ani-tui-windows-x86_64.zip' -OutFile '%TEMP%\ani-tui.zip' -UseBasicParsing"

if exist "%TEMP%\ani-tui.zip" (
    echo Extracting ani-tui...
    powershell -Command "Expand-Archive -Path '%TEMP%\ani-tui.zip' -DestinationPath '%INSTALL_DIR%' -Force"
    del /f "%TEMP%\ani-tui.zip" 2>nul
    echo ✓ ani-tui extracted
) else (
    echo ✗ Failed to download ani-tui
    pause
    exit /b 1
)

echo.
echo [4/4] Setting up PATH...

REM Add to PATH
powershell -Command "[Environment]::SetEnvironmentVariable('PATH', [Environment]::GetEnvironmentVariable('PATH', 'User') + ';%INSTALL_DIR%', 'User')"

REM Create batch wrapper for immediate use
echo @echo off > "%INSTALL_DIR%\ani-tui.cmd"
echo "%~dp0ani-tui.exe" %%* >> "%INSTALL_DIR%\ani-tui.cmd"

echo ✓ PATH updated

echo.
echo ============================================
echo  Installation Complete!
echo ============================================
echo.
echo Installed to: %INSTALL_DIR%
echo.
echo ⚠️  IMPORTANT: You MUST open a NEW terminal window
echo     for the 'ani-tui' command to work!
echo.
echo After opening new terminal, run:
echo   ani-tui              - Start the app
echo   ani-tui -q "naruto"  - Search immediately
echo.
echo Or run directly now (no new terminal needed):
echo   %INSTALL_DIR%\ani-tui.exe
echo.

if not exist "%INSTALL_DIR%\mpv" (
    if not exist "%INSTALL_DIR%\mpv\*" (
        echo ⚠️  WARNING: mpv may not be installed correctly.
        echo     Videos will NOT play without mpv!
        echo     Download from: https://mpv.io/installation/
        echo.
    )
)

pause