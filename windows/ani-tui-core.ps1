<#
.SYNOPSIS
    ani-tui v5.5 for Windows - Core Logic
.DESCRIPTION
    PowerShell core for ani-tui. Uses external batch scripts for fast fzf callbacks.
#>

param(
    [Parameter(Position=0)][string]$Cmd = "",
    [Parameter(ValueFromRemainingArguments)][string[]]$Args
)

$ErrorActionPreference = "SilentlyContinue"

# =============================================================================
# CONFIG
# =============================================================================
$VERSION = "5.5.0"
$DATA = "$env:USERPROFILE\.ani-tui"
$CACHE = "$DATA\cache"
$IMAGES = "$CACHE\images"
$HISTORY = "$DATA\history.json"
$API = "https://api.allanime.day"
$REFR = "https://allmanga.to"
$ANILIST = "https://graphql.anilist.co"

# Colors for fzf
$CLR1 = "fg:#cdd6f4,bg:#1e1e2e,hl:#f9e2af,fg+:#cdd6f4,bg+:#313244,hl+:#f9e2af,info:#94e2d5,prompt:#f5c2e7,pointer:#f5e0dc,marker:#a6e3a1,spinner:#f5e0dc,header:#89b4fa,border:#6c7086"
$CLR2 = "fg:#cdd6f4,bg:#1e1e2e,hl:#a6e3a1,fg+:#cdd6f4,bg+:#313244,hl+:#a6e3a1,prompt:#94e2d5,pointer:#f5e0dc,header:#cba6f7,border:#6c7086"

# =============================================================================
# SETUP
# =============================================================================
function Setup {
    foreach ($d in @($DATA, $CACHE, $IMAGES)) {
        if (!(Test-Path $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null }
    }
    if (!(Test-Path $HISTORY)) { "[]" | Out-File $HISTORY -Encoding UTF8 }
    
    # Create helper batch scripts (fast startup for fzf callbacks)
    CreateHelperScripts
}

function CreateHelperScripts {
    # Search helper - called by fzf on change
    $search = @'
@echo off
setlocal
set "q=%*"
if "%q%"=="" goto :history
if "%q:~1%"=="" goto :history
goto :search

:history
powershell -NoProfile -NoLogo -Command "try{(Get-Content '%USERPROFILE%\.ani-tui\history.json'|ConvertFrom-Json)|Select -First 10|%%{\"HIST`t[$($_.last_episode)] $($_.title)\"}}catch{}"
goto :eof

:search
powershell -NoProfile -NoLogo -Command "$q='%q%';$g='query($search:SearchInput$limit:Int$page:Int$translationType:VaildTranslationTypeEnumType$countryOrigin:VaildCountryOriginEnumType){shows(search:$search limit:$limit page:$page translationType:$translationType countryOrigin:$countryOrigin){edges{_id name availableEpisodes}}}';$v='{\"search\":{\"allowAdult\":false,\"allowUnknown\":false,\"query\":\"'+$q+'\"},\"limit\":30,\"page\":1,\"translationType\":\"sub\",\"countryOrigin\":\"ALL\"}';try{$r=Invoke-RestMethod \"https://api.allanime.day/api?variables=$([uri]::EscapeDataString($v))&query=$([uri]::EscapeDataString($g))\" -Headers @{'User-Agent'='Mozilla/5.0';'Referer'='https://allmanga.to'} -TimeoutSec 10;$r.data.shows.edges|?{$_.availableEpisodes.sub -gt 0}|%%{\"$($_._id)`t$($_.name) ($($_.availableEpisodes.sub) eps)\"}}catch{}"
'@
    $search | Out-File "$CACHE\search.cmd" -Encoding ASCII

    # Preview helper - shows anime cover with chafa
    $preview = @'
@echo off
setlocal enabledelayedexpansion
set "input=%*"
if "%input%"=="" exit /b

REM Clean title
set "name=%input%"
set "name=!name:HIST	=!"
for /f "tokens=1,* delims=]" %%a in ("!name!") do set "name=%%b"
set "name=!name:~1!"

REM Show title
echo.
echo   !name!
echo.

REM Try to show image with chafa
where chafa >nul 2>&1
if errorlevel 1 (
    echo   [Install chafa for image preview: scoop install chafa]
    exit /b
)

REM Calculate hash and check cache
for /f %%h in ('powershell -NoProfile -NoLogo -Command "[BitConverter]::ToString([Security.Cryptography.MD5]::Create().ComputeHash([Text.Encoding]::UTF8.GetBytes('%name%'))).Replace('-','').Substring(0,12).ToLower()"') do set "hash=%%h"
set "img=%USERPROFILE%\.ani-tui\cache\images\!hash!.jpg"

if not exist "!img!" (
    REM Fetch from AniList
    powershell -NoProfile -NoLogo -Command "$n='%name%';$b=@{query=\"query{Page(perPage:1){media(search:\`\"$n\`\",type:ANIME){coverImage{large}}}}\"};try{$r=Invoke-RestMethod 'https://graphql.anilist.co' -Method Post -ContentType 'application/json' -Body ($b|ConvertTo-Json) -TimeoutSec 5;if($r.data.Page.media[0].coverImage.large){Invoke-WebRequest $r.data.Page.media[0].coverImage.large -OutFile '%img%' -TimeoutSec 10}}catch{}"
)

if exist "!img!" (
    chafa --size=50x30 "!img!" 2>nul
)
'@
    $preview | Out-File "$CACHE\preview.cmd" -Encoding ASCII

    # Delete helper
    $delete = @'
@echo off
set "t=%*"
for /f "tokens=1,* delims=]" %%a in ("%t%") do set "t=%%b"
set "t=%t:~1%"
powershell -NoProfile -NoLogo -Command "$t='%t%';$h=@((Get-Content '%USERPROFILE%\.ani-tui\history.json'|ConvertFrom-Json)|?{$_.title -ne $t});$h|ConvertTo-Json -Depth 10|Out-File '%USERPROFILE%\.ani-tui\history.json' -Encoding UTF8"
'@
    $delete | Out-File "$CACHE\delete.cmd" -Encoding ASCII

    # History helper
    $hist = @'
@echo off
powershell -NoProfile -NoLogo -Command "try{(Get-Content '%USERPROFILE%\.ani-tui\history.json'|ConvertFrom-Json)|Select -First 10|%%{\"HIST`t[$($_.last_episode)] $($_.title)\"}}catch{}"
'@
    $hist | Out-File "$CACHE\history.cmd" -Encoding ASCII
}

# =============================================================================
# HISTORY
# =============================================================================
function GetHistory {
    try {
        $c = Get-Content $HISTORY -Raw
        if (!$c) { return @() }
        $h = $c | ConvertFrom-Json
        if ($h -isnot [array]) { @($h) } else { $h }
    } catch { @() }
}

function SaveHistory($title, $ep) {
    $ts = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $h = @(GetHistory)
    $found = $false
    for ($i=0; $i -lt $h.Count; $i++) {
        if ($h[$i].title -eq $title) {
            $h[$i].last_episode = $ep
            $h[$i].last_watched = $ts
            $found = $true
            break
        }
    }
    if (!$found) { $h += [PSCustomObject]@{title=$title;last_episode=$ep;last_watched=$ts} }
    $h | ConvertTo-Json -Depth 10 | Out-File $HISTORY -Encoding UTF8
}

# =============================================================================
# API
# =============================================================================
function SearchAnime($q) {
    if (!$q -or $q.Length -lt 2) { return @() }
    $gql = 'query($search:SearchInput$limit:Int$page:Int$translationType:VaildTranslationTypeEnumType$countryOrigin:VaildCountryOriginEnumType){shows(search:$search limit:$limit page:$page translationType:$translationType countryOrigin:$countryOrigin){edges{_id name availableEpisodes}}}'
    $v = (@{search=@{allowAdult=$false;allowUnknown=$false;query=$q};limit=30;page=1;translationType="sub";countryOrigin="ALL"} | ConvertTo-Json -Compress)
    try {
        $r = Invoke-RestMethod "$API/api?variables=$([uri]::EscapeDataString($v))&query=$([uri]::EscapeDataString($gql))" -Headers @{"User-Agent"="Mozilla/5.0";"Referer"=$REFR} -TimeoutSec 15
        $results = @()
        foreach ($s in $r.data.shows.edges) {
            if ($s.availableEpisodes.sub -gt 0) {
                $results += [PSCustomObject]@{id=$s._id;name=$s.name;eps=$s.availableEpisodes.sub}
            }
        }
        $results
    } catch { @() }
}

function GetEpisodes($id) {
    $gql = 'query($showId:String!){show(_id:$showId){availableEpisodesDetail}}'
    $v = (@{showId=$id} | ConvertTo-Json -Compress)
    try {
        $r = Invoke-RestMethod "$API/api?variables=$([uri]::EscapeDataString($v))&query=$([uri]::EscapeDataString($gql))" -Headers @{"User-Agent"="Mozilla/5.0";"Referer"=$REFR} -TimeoutSec 15
        $r.data.show.availableEpisodesDetail.sub | Sort-Object {[double]$_}
    } catch { @() }
}

# =============================================================================
# MAIN TUI
# =============================================================================
function RunTUI {
    Setup
    
    # Check fzf
    if (!(Get-Command fzf -ErrorAction SilentlyContinue)) {
        Write-Host "`nERROR: fzf not found!" -ForegroundColor Red
        Write-Host "Install: scoop install fzf`n" -ForegroundColor Yellow
        exit 1
    }
    
    while ($true) {
        # Get initial history
        $items = @()
        GetHistory | Select-Object -First 10 | ForEach-Object {
            $items += "HIST`t[$($_.last_episode)] $($_.title)"
        }
        
        # Main fzf with real-time search
        $header = "ani-tui v$VERSION | Type=Search | Enter=Select | Ctrl-D=Delete | Esc=Quit"
        $searchCmd = "`"$CACHE\search.cmd`" {q}"
        $histCmd = "`"$CACHE\history.cmd`""
        $delCmd = "`"$CACHE\delete.cmd`" {2}"
        $prevCmd = "`"$CACHE\preview.cmd`" {2}"
        
        $sel = ($items -join "`n") | fzf --ansi --height=100% --layout=reverse --border=rounded --padding=1 `
            --delimiter="`t" --with-nth=2 `
            --header="$header" --header-first `
            --prompt="Search: " --pointer=">" `
            --preview="$prevCmd" --preview-window="right:50%:wrap" `
            --bind="change:reload:$searchCmd" `
            --bind="ctrl-d:execute-silent($delCmd)+reload($histCmd)" `
            --color="$CLR1" 2>$null
        
        if (!$sel) { break }
        
        # Parse selection
        $parts = $sel -split "`t", 2
        $showId = $null
        $title = $null
        
        if ($sel -match "^HIST") {
            # From history
            $title = ($parts[1] -replace '^\[\d+\]\s*','').Trim()
            Write-Host "`nSearching: $title" -ForegroundColor Cyan
            $results = @(SearchAnime $title)
            if ($results.Count -gt 0) { $showId = $results[0].id }
        } else {
            # From search
            $showId = $parts[0]
            $title = ($parts[1] -replace '\s*\(\d+\s+eps\)$','').Trim()
        }
        
        if (!$showId) {
            Write-Host "Could not find anime" -ForegroundColor Yellow
            Start-Sleep 1
            continue
        }
        
        # Get episodes
        Write-Host "Loading episodes..." -ForegroundColor Cyan
        $episodes = @(GetEpisodes $showId)
        
        if ($episodes.Count -eq 0) {
            Write-Host "No episodes available" -ForegroundColor Yellow
            Start-Sleep 1
            continue
        }
        
        # Get last watched
        $lastEp = 0
        GetHistory | ForEach-Object { if ($_.title -eq $title) { $lastEp = [int]$_.last_episode } }
        
        # Format episodes
        $epItems = foreach ($ep in $episodes) {
            $n = [int]$ep
            if ($n -eq $lastEp) { "$ep  << Last watched" }
            elseif ($n -eq $lastEp + 1) { "$ep  >> Continue" }
            else { "$ep" }
        }
        
        # Episode selection
        $epHeader = "$title`n`nEnter=Play | Esc=Back"
        $epSel = $epItems | fzf --ansi --height=100% --layout=reverse --border=rounded --padding=1 `
            --header="$epHeader" --header-first --prompt="Episode: " --pointer=">" --no-info `
            --color="$CLR2"
        
        if (!$epSel) { continue }
        
        $episode = [int](($epSel -split '\s+')[0])
        
        # Save history
        SaveHistory $title $episode
        
        # Play with ani-cli
        Clear-Host
        Write-Host ""
        Write-Host "  >> Now Playing: $title - Episode $episode" -ForegroundColor Green
        Write-Host ""
        
        if (Get-Command ani-cli -ErrorAction SilentlyContinue) {
            & ani-cli -S 1 -e $episode "$title"
        } else {
            Write-Host "  ani-cli not found." -ForegroundColor Yellow
            Write-Host "  Install: scoop bucket add extras" -ForegroundColor Gray
            Write-Host "           scoop install ani-cli mpv" -ForegroundColor Gray
            Write-Host ""
            Write-Host "  Episode saved to history." -ForegroundColor Cyan
            Read-Host "`n  Press Enter"
        }
    }
    
    Write-Host "`n  Goodbye!`n" -ForegroundColor Cyan
}

# =============================================================================
# ENTRY
# =============================================================================
switch ($Cmd.ToLower()) {
    "-h" { Write-Host "`nani-tui v$VERSION`nUsage: ani-tui`nInstall: scoop install fzf chafa ani-cli mpv`n" }
    "--help" { & $PSCommandPath -h }
    "-v" { Write-Host "ani-tui $VERSION" }
    "--version" { & $PSCommandPath -v }
    default { RunTUI }
}
