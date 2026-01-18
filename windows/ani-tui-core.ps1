<#
.SYNOPSIS
    ani-tui v6.1 for Windows - Optimized for Zero Flickering
.DESCRIPTION
    Uses native batch/curl for fzf callbacks to eliminate PowerShell spawn overhead.
    PowerShell only runs once at startup; all fzf callbacks use fast batch scripts.
#>

param(
    [Parameter(Position=0)][string]$Command = "",
    [Parameter(Position=1)][string]$Arg1 = "",
    [Parameter(ValueFromRemainingArguments)][string[]]$RestArgs
)

$ErrorActionPreference = "SilentlyContinue"

# =============================================================================
# CONFIG
# =============================================================================
$script:VERSION = "6.1.0"
$script:DATA = "$env:USERPROFILE\.ani-tui"
$script:CACHE = "$script:DATA\cache"
$script:IMAGES = "$script:CACHE\images"
$script:HISTORY = "$script:DATA\history.json"
$script:SCRIPTS = "$script:CACHE\scripts"
$script:API = "https://api.allanime.day"
$script:REFR = "https://allmanga.to"
$script:ANILIST = "https://graphql.anilist.co"

# Colors for fzf (Catppuccin Mocha theme)
$script:CLR_MAIN = "fg:#cdd6f4,bg:#1e1e2e,hl:#f9e2af,fg+:#cdd6f4,bg+:#313244,hl+:#f9e2af,info:#94e2d5,prompt:#f5c2e7,pointer:#f5e0dc,marker:#a6e3a1,spinner:#f5e0dc,header:#89b4fa,border:#6c7086"
$script:CLR_EP = "fg:#cdd6f4,bg:#1e1e2e,hl:#a6e3a1,fg+:#cdd6f4,bg+:#313244,hl+:#a6e3a1,prompt:#94e2d5,pointer:#f5e0dc,header:#cba6f7,border:#6c7086"

# =============================================================================
# SETUP - Create directories and helper scripts
# =============================================================================
function Initialize {
    foreach ($dir in @($script:DATA, $script:CACHE, $script:IMAGES, $script:SCRIPTS)) {
        if (!(Test-Path $dir)) { 
            New-Item -ItemType Directory -Path $dir -Force | Out-Null 
        }
    }
    if (!(Test-Path $script:HISTORY)) { 
        "[]" | Out-File $script:HISTORY -Encoding UTF8 
    }
    
    # Create optimized batch helper scripts (these use curl.exe, not PowerShell)
    Create-HelperScripts
}

function Create-HelperScripts {
    # ==========================================================================
    # SEARCH HELPER - Uses curl.exe directly (no PowerShell spawn!)
    # ==========================================================================
    $searchScript = @'
@echo off
setlocal enabledelayedexpansion
set "q=%*"
set "HIST=%USERPROFILE%\.ani-tui\history.json"
set "CACHE=%USERPROFILE%\.ani-tui\cache"

REM If query is empty or 1 char, show history
if "%q%"=="" goto :show_history
set "len=0"
set "tmp=%q%"
:strlen_loop
if not "!tmp!"=="" (set /a len+=1 & set "tmp=!tmp:~1!" & goto :strlen_loop)
if %len% LSS 2 goto :show_history
goto :do_search

:show_history
REM Parse history.json with simple batch (fast!)
if not exist "%HIST%" exit /b
for /f "usebackq tokens=*" %%a in ("%HIST%") do set "json=%%a"
REM Simple extraction using PowerShell (but only for history, not hot path)
powershell -NoLogo -NoProfile -Command "$h=Get-Content '%HIST%'|ConvertFrom-Json;$h|Select -First 10|%%{\"HIST`t[$($_.last_episode)] $($_.title)\"}"
exit /b

:do_search
REM Build API URL - use curl.exe directly
set "gql=query($search:SearchInput$limit:Int$page:Int$translationType:VaildTranslationTypeEnumType$countryOrigin:VaildCountryOriginEnumType){shows(search:$search limit:$limit page:$page translationType:$translationType countryOrigin:$countryOrigin){edges{_id name availableEpisodes}}}"
set "vars={\"search\":{\"allowAdult\":false,\"allowUnknown\":false,\"query\":\"%q%\"},\"limit\":30,\"page\":1,\"translationType\":\"sub\",\"countryOrigin\":\"ALL\"}"

REM URL encode the query (simplified)
set "url=https://api.allanime.day/api"

REM Use curl and parse with PowerShell (one call)
curl.exe -s -G "%url%" --data-urlencode "variables=%vars%" --data-urlencode "query=%gql%" -H "User-Agent: Mozilla/5.0" -H "Referer: https://allmanga.to" 2>nul | powershell -NoLogo -NoProfile -Command "$r=[Console]::In.ReadToEnd()|ConvertFrom-Json;$r.data.shows.edges|?{$_.availableEpisodes.sub -gt 0}|%%{\"$($_._id)`t$($_.name) ($($_.availableEpisodes.sub) eps)\"}"
exit /b
'@
    $searchScript | Out-File "$script:SCRIPTS\search.cmd" -Encoding ASCII

    # ==========================================================================
    # HISTORY HELPER - Simple PowerShell call
    # ==========================================================================
    $historyScript = @'
@echo off
powershell -NoLogo -NoProfile -Command "$h=Get-Content '%USERPROFILE%\.ani-tui\history.json' -ErrorAction SilentlyContinue|ConvertFrom-Json;if($h){$h|Select -First 10|%%{\"HIST`t[$($_.last_episode)] $($_.title)\"}}"
'@
    $historyScript | Out-File "$script:SCRIPTS\history.cmd" -Encoding ASCII

    # ==========================================================================
    # DELETE HELPER
    # ==========================================================================
    $deleteScript = @'
@echo off
setlocal enabledelayedexpansion
set "input=%*"
set "HIST=%USERPROFILE%\.ani-tui\history.json"

REM Extract title (remove [xx] prefix)
for /f "tokens=1,* delims=]" %%a in ("%input%") do set "title=%%b"
set "title=!title:~1!"

REM Delete from history using PowerShell
powershell -NoLogo -NoProfile -Command "$t='%title%';$h=@((Get-Content '%HIST%'|ConvertFrom-Json)|?{$_.title -ne $t});if($h.Count -eq 0){'[]'}else{$h|ConvertTo-Json -Depth 10}|Out-File '%HIST%' -Encoding UTF8"
'@
    $deleteScript | Out-File "$script:SCRIPTS\delete.cmd" -Encoding ASCII

    # ==========================================================================
    # PREVIEW HELPER - Shows anime title and cover image
    # Uses curl.exe for fetching, chafa for display
    # ==========================================================================
    $previewScript = @'
@echo off
setlocal enabledelayedexpansion
set "input=%*"
if "%input%"=="" exit /b

set "IMAGES=%USERPROFILE%\.ani-tui\cache\images"
if not exist "%IMAGES%" mkdir "%IMAGES%"

REM Clean the title
set "name=%input%"
set "name=!name:HIST	=!"
for /f "tokens=1,* delims=]" %%a in ("!name!") do (
    if not "%%b"=="" set "name=%%b"
)
if "!name:~0,1!"==" " set "name=!name:~1!"

REM Remove episode count suffix like "(123 eps)"
for /f "tokens=1 delims=(" %%a in ("!name!") do set "name=%%a"
REM Trim trailing space
if "!name:~-1!"==" " set "name=!name:~0,-1!"

if "!name!"=="" exit /b

REM Display title
echo.
echo   !name!
echo.

REM Check for chafa
where chafa >nul 2>&1
if errorlevel 1 (
    echo   [Install chafa for image preview]
    echo   scoop install chafa
    exit /b
)

REM Generate hash for cache filename (use certutil)
echo !name!>"%TEMP%\ani_hash_input.txt"
for /f "skip=1 tokens=*" %%h in ('certutil -hashfile "%TEMP%\ani_hash_input.txt" MD5 2^>nul ^| findstr /v ":"') do (
    set "hash=%%h"
    goto :hash_done
)
:hash_done
set "hash=!hash: =!"
set "hash=!hash:~0,12!"
set "imgfile=%IMAGES%\!hash!.jpg"

REM If image not cached, fetch from AniList
if not exist "!imgfile!" (
    REM Build GraphQL query
    set "query={\"query\":\"query{Page(perPage:1){media(search:\\\"!name!\\\",type:ANIME){coverImage{large}}}}\"}"
    
    REM Fetch cover URL and download image
    for /f "usebackq delims=" %%u in (`curl.exe -s -X POST "https://graphql.anilist.co" -H "Content-Type: application/json" -d "!query!" 2^>nul ^| powershell -NoLogo -NoProfile -Command "$r=[Console]::In.ReadToEnd()|ConvertFrom-Json;$r.data.Page.media[0].coverImage.large"`) do (
        if not "%%u"=="" if not "%%u"=="null" (
            curl.exe -sL "%%u" -o "!imgfile!" 2>nul
        )
    )
)

REM Display image with chafa
if exist "!imgfile!" (
    chafa --size=50x30 "!imgfile!" 2>nul
)
exit /b
'@
    $previewScript | Out-File "$script:SCRIPTS\preview.cmd" -Encoding ASCII
}

# =============================================================================
# HISTORY FUNCTIONS
# =============================================================================
function Get-AnimeHistory {
    try {
        $content = Get-Content $script:HISTORY -Raw -ErrorAction Stop
        if ([string]::IsNullOrWhiteSpace($content)) { return @() }
        $parsed = $content | ConvertFrom-Json
        if ($parsed -isnot [array]) { return @($parsed) }
        return $parsed
    } catch { 
        return @() 
    }
}

function Save-AnimeHistory($title, $episode) {
    $timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $history = @(Get-AnimeHistory)
    $found = $false
    
    for ($i = 0; $i -lt $history.Count; $i++) {
        if ($history[$i].title -eq $title) {
            $history[$i].last_episode = [int]$episode
            $history[$i].last_watched = $timestamp
            $found = $true
            break
        }
    }
    
    if (!$found) {
        $history += [PSCustomObject]@{
            title = $title
            last_episode = [int]$episode
            last_watched = $timestamp
        }
    }
    
    $history | ConvertTo-Json -Depth 10 | Out-File $script:HISTORY -Encoding UTF8
}

# =============================================================================
# API FUNCTIONS (used only in main PowerShell process)
# =============================================================================
function Search-Anime($query) {
    if (!$query -or $query.Length -lt 2) { return @() }
    
    $gql = 'query($search:SearchInput$limit:Int$page:Int$translationType:VaildTranslationTypeEnumType$countryOrigin:VaildCountryOriginEnumType){shows(search:$search limit:$limit page:$page translationType:$translationType countryOrigin:$countryOrigin){edges{_id name availableEpisodes}}}'
    $vars = @{
        search = @{ allowAdult = $false; allowUnknown = $false; query = $query }
        limit = 30
        page = 1
        translationType = "sub"
        countryOrigin = "ALL"
    } | ConvertTo-Json -Compress
    
    try {
        $url = "$script:API/api?variables=$([uri]::EscapeDataString($vars))&query=$([uri]::EscapeDataString($gql))"
        $response = Invoke-RestMethod $url -Headers @{
            "User-Agent" = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
            "Referer" = $script:REFR
        } -TimeoutSec 15
        
        $results = @()
        foreach ($show in $response.data.shows.edges) {
            if ($show.availableEpisodes.sub -gt 0) {
                $results += [PSCustomObject]@{
                    id = $show._id
                    name = $show.name
                    eps = $show.availableEpisodes.sub
                }
            }
        }
        return $results
    } catch {
        return @()
    }
}

function Get-Episodes($showId) {
    $gql = 'query($showId:String!){show(_id:$showId){availableEpisodesDetail}}'
    $vars = (@{ showId = $showId } | ConvertTo-Json -Compress)
    
    try {
        $url = "$script:API/api?variables=$([uri]::EscapeDataString($vars))&query=$([uri]::EscapeDataString($gql))"
        $response = Invoke-RestMethod $url -Headers @{
            "User-Agent" = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
            "Referer" = $script:REFR
        } -TimeoutSec 15
        
        return $response.data.show.availableEpisodesDetail.sub | Sort-Object { [double]$_ }
    } catch {
        return @()
    }
}

# =============================================================================
# MAIN TUI
# =============================================================================
function Start-TUI {
    Initialize
    
    # Check for fzf
    if (!(Get-Command fzf -ErrorAction SilentlyContinue)) {
        Write-Host ""
        Write-Host "  ERROR: fzf not found!" -ForegroundColor Red
        Write-Host "  Install: scoop install fzf" -ForegroundColor Yellow
        Write-Host ""
        exit 1
    }
    
    # Check for curl.exe (required for fast callbacks)
    if (!(Get-Command curl.exe -ErrorAction SilentlyContinue)) {
        Write-Host ""
        Write-Host "  ERROR: curl.exe not found!" -ForegroundColor Red
        Write-Host "  Windows 10+ should have curl.exe built-in." -ForegroundColor Yellow
        Write-Host ""
        exit 1
    }
    
    while ($true) {
        # Get initial items (history)
        $items = @()
        Get-AnimeHistory | Select-Object -First 10 | ForEach-Object {
            $items += "HIST`t[$($_.last_episode)] $($_.title)"
        }
        
        # Use batch scripts for fzf callbacks (fast, no PowerShell spawn!)
        $searchCmd = "`"$script:SCRIPTS\search.cmd`" {q}"
        $historyCmd = "`"$script:SCRIPTS\history.cmd`""
        $deleteCmd = "`"$script:SCRIPTS\delete.cmd`" {2}"
        $previewCmd = "`"$script:SCRIPTS\preview.cmd`" {2}"
        
        $header = "ani-tui v$script:VERSION | Type=Search | Enter=Select | Ctrl-D=Delete | Esc=Quit"
        
        # Run fzf with batch script callbacks
        $selection = ($items -join "`n") | fzf `
            --ansi `
            --height=100% `
            --layout=reverse `
            --border=rounded `
            --padding=1 `
            --delimiter="`t" `
            --with-nth=2 `
            --header="$header" `
            --header-first `
            --prompt="Search: " `
            --pointer=">" `
            --preview="$previewCmd" `
            --preview-window="right:50%:wrap" `
            --bind="change:reload:$searchCmd" `
            --bind="ctrl-d:execute-silent($deleteCmd)+reload($historyCmd)" `
            --color="$script:CLR_MAIN" 2>$null
        
        if (!$selection) { break }
        
        # Parse selection
        $parts = $selection -split "`t", 2
        $showId = $null
        $title = $null
        
        if ($selection -match "^HIST") {
            # From history - extract title and search for show ID
            $title = ($parts[1] -replace '^\[\d+\]\s*', '').Trim()
            Write-Host ""
            Write-Host "  Searching: $title" -ForegroundColor Cyan
            $results = @(Search-Anime $title)
            if ($results.Count -gt 0) {
                $showId = $results[0].id
            }
        } else {
            # From search results
            $showId = $parts[0]
            $title = ($parts[1] -replace '\s*\(\d+\s+eps\)\s*$', '').Trim()
        }
        
        if (!$showId) {
            Write-Host "  Could not find anime" -ForegroundColor Yellow
            Start-Sleep -Seconds 1
            continue
        }
        
        # Get episodes
        Write-Host "  Loading episodes..." -ForegroundColor Cyan
        $episodes = @(Get-Episodes $showId)
        
        if ($episodes.Count -eq 0) {
            Write-Host "  No episodes available" -ForegroundColor Yellow
            Start-Sleep -Seconds 1
            continue
        }
        
        # Get last watched episode
        $lastEp = 0
        Get-AnimeHistory | ForEach-Object {
            if ($_.title -eq $title) { $lastEp = [int]$_.last_episode }
        }
        
        # Format episode list with markers
        $epItems = foreach ($ep in $episodes) {
            $epNum = [int]$ep
            if ($epNum -eq $lastEp) {
                "$ep  << Last watched"
            } elseif ($epNum -eq ($lastEp + 1)) {
                "$ep  >> Continue"
            } else {
                "$ep"
            }
        }
        
        # Episode selection fzf (no preview needed, simple list)
        $epHeader = "$title`n`nEnter=Play | Esc=Back"
        $epSelection = $epItems | fzf `
            --ansi `
            --height=100% `
            --layout=reverse `
            --border=rounded `
            --padding=1 `
            --header="$epHeader" `
            --header-first `
            --prompt="Episode: " `
            --pointer=">" `
            --no-info `
            --color="$script:CLR_EP"
        
        if (!$epSelection) { continue }
        
        # Parse episode number
        $episode = [int](($epSelection -split '\s+')[0])
        
        # Save to history
        Save-AnimeHistory $title $episode
        
        # Play with ani-cli
        Clear-Host
        Write-Host ""
        Write-Host "  >> Now Playing: $title - Episode $episode" -ForegroundColor Green
        Write-Host ""
        
        if (Get-Command ani-cli -ErrorAction SilentlyContinue) {
            & ani-cli -S 1 -e $episode "$title"
        } else {
            Write-Host "  ani-cli not found." -ForegroundColor Yellow
            Write-Host ""
            Write-Host "  Install streaming support:" -ForegroundColor Gray
            Write-Host "    scoop bucket add extras" -ForegroundColor DarkGray
            Write-Host "    scoop install ani-cli mpv" -ForegroundColor DarkGray
            Write-Host ""
            Write-Host "  Episode saved to history." -ForegroundColor Cyan
            Write-Host ""
            Read-Host "  Press Enter to continue"
        }
    }
    
    Write-Host ""
    Write-Host "  Goodbye!" -ForegroundColor Cyan
    Write-Host ""
}

# =============================================================================
# ENTRY POINT
# =============================================================================
switch ($Command.ToLower()) {
    "-h" {
        Write-Host ""
        Write-Host "  ani-tui v$script:VERSION - Anime TUI for Windows"
        Write-Host ""
        Write-Host "  Usage: ani-tui"
        Write-Host ""
        Write-Host "  Controls:"
        Write-Host "    Type      Search anime (real-time)"
        Write-Host "    Up/Down   Navigate"
        Write-Host "    Enter     Select/Play"
        Write-Host "    Ctrl-D    Delete from history"
        Write-Host "    Esc       Back/Quit"
        Write-Host ""
        Write-Host "  Dependencies:"
        Write-Host "    scoop install fzf chafa ani-cli mpv"
        Write-Host ""
    }
    "--help" { & $PSCommandPath -h }
    "-v" { Write-Host "ani-tui $script:VERSION" }
    "--version" { & $PSCommandPath -v }
    default { Start-TUI }
}
