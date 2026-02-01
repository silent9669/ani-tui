@echo off
setlocal EnableDelayedExpansion

echo ============================================
echo  ani-tui Complete Windows Installer
echo ============================================
echo.
echo This installer will set up:
echo  - Visual C++ Redistributable (REQUIRED)
echo  - mpv (for video playback)
echo  - chafa (for image previews)  
echo  - ani-tui
echo.
echo Press any key to continue...
pause > nul

set "INSTALL_DIR=%USERPROFILE%\ani-tui"

REM Check for Visual C++ Redistributable
echo.
echo [Step 1/5] Checking Visual C++ Redistributable...
echo This is REQUIRED for ani-tui to run!

if exist "%SystemRoot%\System32\vcruntime140.dll" (
    echo [OK] Visual C++ Redistributable found
) else (
    echo [!] Visual C++ Redistributable not found
    echo Installing via winget...
    winget install Microsoft.VCRedist.2015+.x64 --accept-source-agreements --accept-package-agreements
    if %errorLevel% neq 0 (
        echo [!] Could not auto-install
        echo Please download and install manually:
        echo https://aka.ms/vs/17/release/vc_redist.x64.exe
        echo.
        pause
    )
)

REM Create install directory
if not exist "%INSTALL_DIR%" (
    mkdir "%INSTALL_DIR%"
)

REM Install mpv
echo.
echo [Step 2/5] Installing mpv (REQUIRED for video)...

where mpv > nul 2>&1
if %errorLevel% equ 0 (
    echo [OK] mpv already installed
) else (
    echo Installing mpv via winget...
    winget install mpv --accept-source-agreements --accept-package-agreements
    if %errorLevel% neq 0 (
        echo [!] winget failed, trying manual download...
        powershell -Command "Invoke-WebRequest -Uri 'https://github.com/mpv-player/mpv/releases/download/v0.37.0/mpv-0.37.0-windows-x86_64.zip' -OutFile '%TEMP%\mpv.zip' -UseBasicParsing"
        
        if exist "%TEMP%\mpv.zip" (
            mkdir "%INSTALL_DIR%\mpv" 2> nul
            powershell -Command "Expand-Archive -Path '%TEMP%\mpv.zip' -DestinationPath '%INSTALL_DIR%\mpv' -Force"
            del /f "%TEMP%\mpv.zip" 2> nul
            
            REM Add to PATH
            powershell -Command "[Environment]::SetEnvironmentVariable('PATH', [Environment]::GetEnvironmentVariable('PATH', 'User') + ';%INSTALL_DIR%\mpv', 'User')"
            echo [OK] mpv installed (portable)
        ) else (
            echo [X] Could not install mpv
            echo Please install manually from: https://mpv.io/installation/
        )
    ) else (
        echo [OK] mpv installed via winget
    )
)

REM Install chafa
echo.
echo [Step 3/5] Installing chafa (optional, for images)...

where chafa > nul 2>&1
if %errorLevel% equ 0 (
    echo [OK] chafa already installed
) else (
    winget install hpjansson.chafa --accept-source-agreements --accept-package-agreements
    if %errorLevel% equ 0 (
        echo [OK] chafa installed
    ) else (
        echo [!] Could not install chafa (optional)
    )
)

REM Install ani-tui
echo.
echo [Step 4/5] Installing ani-tui...

powershell -Command "Invoke-WebRequest -Uri 'https://github.com/silent9669/ani-tui/releases/latest/download/ani-tui-windows-x86_64.zip' -OutFile '%TEMP%\ani-tui.zip' -UseBasicParsing"

if exist "%TEMP%\ani-tui.zip" (
    echo Extracting ani-tui...
    powershell -Command "Expand-Archive -Path '%TEMP%\ani-tui.zip' -DestinationPath '%INSTALL_DIR%' -Force"
    del /f "%TEMP%\ani-tui.zip" 2> nul
    echo [OK] ani-tui extracted
) else (
    echo [X] Failed to download ani-tui
    pause
    exit /b 1
)

REM Setup PATH
echo.
echo [Step 5/5] Setting up PATH...

powershell -Command "[Environment]::SetEnvironmentVariable('PATH', [Environment]::GetEnvironmentVariable('PATH', 'User') + ';%INSTALL_DIR%', 'User')"

REM Create wrapper
echo @echo off > "%INSTALL_DIR%\ani-tui.cmd"
echo "%~dp0ani-tui.exe" %%* >> "%INSTALL_DIR%\ani-tui.cmd"

echo [OK] Setup complete

echo.
echo ============================================
echo  Installation Complete!
echo ============================================
echo.
echo Installed to: %INSTALL_DIR%
echo.
echo !!! IMPORTANT !!!
echo You MUST restart your computer before using ani-tui!
echo.
echo After restart, open a NEW terminal and run:
echo   ani-tui
echo.
echo Or run now without restart:
echo   %INSTALL_DIR%\ani-tui.exe
echo.

where mpv > nul 2>&1
if %errorLevel% neq 0 (
    echo [!] WARNING: mpv not found - videos will NOT play!
)

pause