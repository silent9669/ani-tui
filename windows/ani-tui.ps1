<#
.SYNOPSIS
    ani-tui v5.2 - Enhanced Anime TUI for Windows
    
.DESCRIPTION
    Terminal-based anime browser with fzf interface, image previews, and streaming.
    Windows PowerShell implementation matching macOS functionality.
#>

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Command = "",
    
    [Parameter(Position = 1, ValueFromRemainingArguments)]
    [string[]]$Arguments
)

# =============================================================================
# CONFIGURATION
# =============================================================================

$script:VERSION = "5.2.0"

# Directories
$script:DATA_DIR = Join-Path $env:USERPROFILE ".ani-tui"
$script:CACHE_DIR = Join-Path $script:DATA_DIR "cache"
$script:IMAGES_DIR = Join-Path $script:CACHE_DIR "images"
$script:HISTORY_FILE = Join-Path $script:DATA_DIR "history.json"

# AllAnime API (same as macOS)
$script:ALLANIME_API = "https://api.allanime.day"
$script:ALLANIME_REFR = "https://allmanga.to"
$script:USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/121.0"

# Catppuccin Mocha colors for fzf
$script:FZF_COLORS = "fg:#cdd6f4,bg:#1e1e2e,hl:#f9e2af,fg+:#cdd6f4,bg+:#313244,hl+:#f9e2af,info:#94e2d5,prompt:#f5c2e7,pointer:#f5e0dc,marker:#a6e3a1,spinner:#f5e0dc,header:#89b4fa,border:#6c7086"
$script:FZF_COLORS_EP = "fg:#cdd6f4,bg:#1e1e2e,hl:#a6e3a1,fg+:#cdd6f4,bg+:#313244,hl+:#a6e3a1,prompt:#94e2d5,pointer:#f5e0dc,header:#cba6f7,border:#6c7086"

# =============================================================================
# INITIALIZATION
# =============================================================================

function Initialize-Directories {
    @($script:DATA_DIR, $script:CACHE_DIR, $script:IMAGES_DIR) | ForEach-Object {
        if (-not (Test-Path $_)) {
            New-Item -ItemType Directory -Path $_ -Force | Out-Null
        }
    }
    if (-not (Test-Path $script:HISTORY_FILE)) {
        "[]" | Out-File -FilePath $script:HISTORY_FILE -Encoding UTF8 -NoNewline
    }
}

function Test-Dependencies {
    if (-not (Get-Command "fzf" -ErrorAction SilentlyContinue)) {
        Write-Host "fzf not found. Install with: scoop install fzf" -ForegroundColor Red
        exit 1
    }
}

# =============================================================================
# ALLANIME API FUNCTIONS
# =============================================================================

function Search-AllAnime {
    param([string]$Query)
    
    if ([string]::IsNullOrWhiteSpace($Query) -or $Query.Length -lt 2) {
        return @()
    }
    
    $gql = 'query($search:SearchInput$limit:Int$page:Int$translationType:VaildTranslationTypeEnumType$countryOrigin:VaildCountryOriginEnumType){shows(search:$search limit:$limit page:$page translationType:$translationType countryOrigin:$countryOrigin){edges{_id name availableEpisodes __typename}}}'
    
    $variables = @{
        search = @{
            allowAdult = $false
            allowUnknown = $false
            query = $Query
        }
        limit = 30
        page = 1
        translationType = "sub"
        countryOrigin = "ALL"
    } | ConvertTo-Json -Compress
    
    $uri = "$script:ALLANIME_API/api?variables=$([uri]::EscapeDataString($variables))&query=$([uri]::EscapeDataString($gql))"
    
    try {
        $response = Invoke-RestMethod -Uri $uri -Headers @{
            "User-Agent" = $script:USER_AGENT
            "Referer" = $script:ALLANIME_REFR
        } -TimeoutSec 10 -ErrorAction Stop
        
        $results = @()
        if ($response.data.shows.edges) {
            foreach ($show in $response.data.shows.edges) {
                $eps = $show.availableEpisodes.sub
                if ($eps -and $eps -gt 0) {
                    $results += "$($show._id)`t$($show.name) ($eps eps)"
                }
            }
        }
        return $results
    }
    catch {
        return @()
    }
}

function Get-Episodes {
    param([string]$ShowId)
    
    $gql = 'query($showId:String!){show(_id:$showId){_id availableEpisodesDetail}}'
    $variables = @{ showId = $ShowId } | ConvertTo-Json -Compress
    $uri = "$script:ALLANIME_API/api?variables=$([uri]::EscapeDataString($variables))&query=$([uri]::EscapeDataString($gql))"
    
    try {
        $response = Invoke-RestMethod -Uri $uri -Headers @{
            "User-Agent" = $script:USER_AGENT
            "Referer" = $script:ALLANIME_REFR
        } -TimeoutSec 10 -ErrorAction Stop
        
        $episodes = @()
        if ($response.data.show.availableEpisodesDetail.sub) {
            $episodes = $response.data.show.availableEpisodesDetail.sub | Sort-Object { [double]$_ }
        }
        return $episodes
    }
    catch {
        return @()
    }
}

# =============================================================================
# HISTORY FUNCTIONS (matching macOS format)
# =============================================================================

function Get-WatchHistory {
    try {
        $content = Get-Content $script:HISTORY_FILE -Raw -ErrorAction Stop
        if ([string]::IsNullOrWhiteSpace($content)) { return @() }
        $history = $content | ConvertFrom-Json
        if ($null -eq $history) { return @() }
        if ($history -isnot [array]) { return @($history) }
        return $history
    }
    catch {
        return @()
    }
}

function Update-WatchHistory {
    param([string]$Title, [int]$Episode)
    
    $timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $history = @(Get-WatchHistory)
    
    $found = $false
    for ($i = 0; $i -lt $history.Count; $i++) {
        if ($history[$i].title -eq $Title) {
            $history[$i].last_episode = $Episode
            $history[$i].last_watched = $timestamp
            $found = $true
            break
        }
    }
    
    if (-not $found) {
        $history += [PSCustomObject]@{
            title = $Title
            last_episode = $Episode
            last_watched = $timestamp
        }
    }
    
    $history | ConvertTo-Json -Depth 10 | Out-File -FilePath $script:HISTORY_FILE -Encoding UTF8
}

function Remove-FromHistory {
    param([string]$Title)
    $history = @(Get-WatchHistory) | Where-Object { $_.title -ne $Title }
    if ($null -eq $history) { $history = @() }
    ConvertTo-Json -InputObject @($history) -Depth 10 | Out-File -FilePath $script:HISTORY_FILE -Encoding UTF8
}

# =============================================================================
# HELPER SCRIPTS CREATION (optimized for less flickering)
# =============================================================================

function Create-HelperScripts {
    # Search helper - uses cmd for faster startup
    $searchBat = @"
@echo off
setlocal enabledelayedexpansion
set "query=%*"
if "%query%"=="" exit /b 0
if "%query:~1%"=="" exit /b 0
powershell -NoProfile -NoLogo -Command "& { `$q='%query%'; `$gql='query(`$search:SearchInput`$limit:Int`$page:Int`$translationType:VaildTranslationTypeEnumType`$countryOrigin:VaildCountryOriginEnumType){shows(search:`$search limit:`$limit page:`$page translationType:`$translationType countryOrigin:`$countryOrigin){edges{_id name availableEpisodes __typename}}}'; `$v='{\"search\":{\"allowAdult\":false,\"allowUnknown\":false,\"query\":\"'+`$q+'\"},\"limit\":30,\"page\":1,\"translationType\":\"sub\",\"countryOrigin\":\"ALL\"}'; try { `$r=Invoke-RestMethod -Uri \"https://api.allanime.day/api?variables=`$([uri]::EscapeDataString(`$v))&query=`$([uri]::EscapeDataString(`$gql))\" -Headers @{'User-Agent'='Mozilla/5.0';'Referer'='https://allmanga.to'} -TimeoutSec 8; `$r.data.shows.edges | ForEach-Object { if(`$_.availableEpisodes.sub -gt 0) { Write-Output \"`$(`$_._id)``t`$(`$_.name) (`$(`$_.availableEpisodes.sub) eps)\" } } } catch {} }"
"@
    $searchBat | Out-File -FilePath "$script:CACHE_DIR\search.bat" -Encoding ASCII
    
    # Preview helper - simple text preview (avoid chafa flickering)
    $previewBat = @"
@echo off
setlocal
set "input=%*"
if "%input%"=="" exit /b 0
echo.
echo   %input%
echo.
if exist "%USERPROFILE%\.ani-tui\cache\images" (
    for /f "tokens=*" %%a in ('powershell -NoProfile -NoLogo -Command "[System.BitConverter]::ToString([System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes('%input%'))).Replace('-','').Substring(0,12).ToLower()"') do set "hash=%%a"
    if exist "%USERPROFILE%\.ani-tui\cache\images\!hash!.jpg" (
        chafa --size=50x30 "%USERPROFILE%\.ani-tui\cache\images\!hash!.jpg" 2>nul
    )
)
"@
    $previewBat | Out-File -FilePath "$script:CACHE_DIR\preview.bat" -Encoding ASCII
    
    # History helper
    $historyBat = @"
@echo off
powershell -NoProfile -NoLogo -Command "& { try { `$h = Get-Content '%USERPROFILE%\.ani-tui\history.json' -Raw | ConvertFrom-Json; `$h | Select-Object -First 10 | ForEach-Object { Write-Output \"HIST``t[`$(`$_.last_episode)] `$(`$_.title)\" } } catch {} }"
"@
    $historyBat | Out-File -FilePath "$script:CACHE_DIR\history.bat" -Encoding ASCII
    
    # Delete helper
    $deleteBat = @"
@echo off
setlocal
set "title=%*"
set "title=%title:~1%"
for /f "tokens=1,* delims=] " %%a in ("%title%") do set "title=%%b"
powershell -NoProfile -NoLogo -Command "& { `$t='%title%'; `$h = @(Get-Content '%USERPROFILE%\.ani-tui\history.json' -Raw | ConvertFrom-Json) | Where-Object { `$_.title -ne `$t }; if(`$null -eq `$h){`$h=@()}; ConvertTo-Json -InputObject @(`$h) -Depth 10 | Out-File '%USERPROFILE%\.ani-tui\history.json' -Encoding UTF8 }"
"@
    $deleteBat | Out-File -FilePath "$script:CACHE_DIR\delete.bat" -Encoding ASCII
}

# =============================================================================
# IMAGE CACHING (background download)
# =============================================================================

function Cache-CoverImage {
    param([string]$Title)
    
    $hash = [System.BitConverter]::ToString(
        [System.Security.Cryptography.MD5]::Create().ComputeHash(
            [System.Text.Encoding]::UTF8.GetBytes($Title)
        )
    ).Replace("-", "").Substring(0, 12).ToLower()
    
    $imgPath = Join-Path $script:IMAGES_DIR "$hash.jpg"
    if (Test-Path $imgPath) { return }
    
    # Fetch cover URL from AniList
    $gql = "query { Page(perPage:1) { media(search:`"$($Title -replace '"', '\"')`",type:ANIME) { coverImage { large } } } }"
    
    try {
        $body = @{ query = $gql } | ConvertTo-Json -Compress
        $response = Invoke-RestMethod -Uri "https://graphql.anilist.co" -Method Post -ContentType "application/json" -Body $body -TimeoutSec 5
        $cover = $response.data.Page.media[0].coverImage.large
        if ($cover) {
            Invoke-WebRequest -Uri $cover -OutFile $imgPath -TimeoutSec 10 | Out-Null
        }
    }
    catch { }
}

# =============================================================================
# MAIN TUI
# =============================================================================

function Start-TUI {
    Initialize-Directories
    Test-Dependencies
    Create-HelperScripts
    
    while ($true) {
        # Build initial list from history
        $history = Get-WatchHistory
        $items = @()
        $history | Select-Object -First 10 | ForEach-Object {
            $items += "HIST`t[$($_.last_episode)] $($_.title)"
            # Cache images in background
            Start-Job -ScriptBlock { param($t,$d) 
                $hash = [System.BitConverter]::ToString([System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($t))).Replace("-","").Substring(0,12).ToLower()
                $img = Join-Path $d "$hash.jpg"
                if (-not (Test-Path $img)) {
                    try {
                        $r = Invoke-RestMethod -Uri "https://graphql.anilist.co" -Method Post -ContentType "application/json" -Body (@{query="query{Page(perPage:1){media(search:`"$t`",type:ANIME){coverImage{large}}}}"} | ConvertTo-Json) -TimeoutSec 5
                        if ($r.data.Page.media[0].coverImage.large) { Invoke-WebRequest -Uri $r.data.Page.media[0].coverImage.large -OutFile $img -TimeoutSec 10 }
                    } catch {}
                }
            } -ArgumentList $_.title, $script:IMAGES_DIR | Out-Null
        }
        
        # Write items to temp file
        $tempFile = Join-Path $env:TEMP "ani-tui-items.txt"
        if ($items.Count -gt 0) {
            $items -join "`n" | Out-File -FilePath $tempFile -Encoding UTF8 -NoNewline
        } else {
            "" | Out-File -FilePath $tempFile -Encoding UTF8 -NoNewline
        }
        
        # Run fzf
        $fzfHeader = "ani-tui v$script:VERSION | Type to search | Enter Select | Ctrl-D Delete | Esc Quit"
        
        $selected = Get-Content $tempFile | fzf `
            --ansi `
            --height=100% `
            --layout=reverse `
            --border=rounded `
            --padding=1 `
            --delimiter="`t" `
            --with-nth=2 `
            --header="$fzfHeader" `
            --header-first `
            --prompt="Search: " `
            --pointer=">" `
            --preview="cmd /c `"$script:CACHE_DIR\preview.bat`" {2}" `
            --preview-window="right:45%:border-rounded:wrap" `
            --bind="change:reload:cmd /c `"$script:CACHE_DIR\search.bat`" {q}" `
            --bind="ctrl-d:execute-silent(cmd /c `"$script:CACHE_DIR\delete.bat`" {2})+reload(cmd /c `"$script:CACHE_DIR\history.bat`")" `
            --color="$script:FZF_COLORS"
        
        # Cleanup background jobs
        Get-Job | Where-Object { $_.State -eq 'Completed' } | Remove-Job -Force
        
        if ([string]::IsNullOrWhiteSpace($selected)) { break }
        
        # Parse selection
        $parts = $selected -split "`t", 2
        $showId = $null
        $title = $null
        
        if ($selected -match "^HIST") {
            # From history
            $title = ($parts[1] -replace '^\[\d+\]\s*', '').Trim()
            Write-Host "`nSearching: $title" -ForegroundColor Cyan
            $searchResult = Search-AllAnime -Query $title
            if ($searchResult.Count -gt 0) {
                $showId = ($searchResult[0] -split "`t")[0]
            }
        }
        else {
            # From search
            $showId = $parts[0]
            $title = ($parts[1] -replace '\s*\(\d+\s+eps\)$', '').Trim()
        }
        
        if ([string]::IsNullOrWhiteSpace($showId)) { 
            Write-Host "Could not find anime" -ForegroundColor Yellow
            Start-Sleep -Seconds 1
            continue 
        }
        
        # Cache cover image for this title
        Cache-CoverImage -Title $title
        
        # Get episodes
        Write-Host "Loading episodes..." -ForegroundColor Cyan
        $episodes = Get-Episodes -ShowId $showId
        
        if ($episodes.Count -eq 0) {
            Write-Host "No episodes available" -ForegroundColor Yellow
            Start-Sleep -Seconds 1
            continue
        }
        
        # Get last watched episode
        $lastEp = 0
        $history | ForEach-Object {
            if ($_.title -eq $title) { $lastEp = [int]$_.last_episode }
        }
        
        # Format episodes with markers
        $epList = @()
        foreach ($ep in $episodes) {
            $epNum = [int]$ep
            if ($epNum -eq $lastEp) {
                $epList += "$ep  << Last watched"
            }
            elseif ($epNum -eq ($lastEp + 1)) {
                $epList += "$ep  >> Continue"
            }
            else {
                $epList += "$ep"
            }
        }
        
        # Episode selection (simple fzf, no reload)
        $epChoice = $epList | fzf `
            --ansi `
            --height=100% `
            --layout=reverse `
            --border=rounded `
            --padding=1 `
            --header="$title`n`nEnter PLAY | Esc BACK" `
            --header-first `
            --prompt="Episode: " `
            --pointer=">" `
            --no-info `
            --color="$script:FZF_COLORS_EP"
        
        if ([string]::IsNullOrWhiteSpace($epChoice)) { continue }
        
        # Extract episode number
        $episode = [int](($epChoice -split '\s+')[0])
        
        # Update history
        Update-WatchHistory -Title $title -Episode $episode
        
        # Play with ani-cli
        Clear-Host
        Write-Host ""
        Write-Host "  > Now Playing: $title - Episode $episode" -ForegroundColor Green
        Write-Host ""
        
        if (Get-Command "ani-cli" -ErrorAction SilentlyContinue) {
            & ani-cli -S 1 -e $episode "$title"
        }
        else {
            Write-Host "  ani-cli not found." -ForegroundColor Yellow
            Write-Host "  Install with: scoop install ani-cli mpv" -ForegroundColor Gray
            Write-Host ""
            Write-Host "  Episode $episode recorded in history." -ForegroundColor Cyan
            Write-Host ""
            Read-Host "  Press Enter to continue"
        }
    }
    
    # Final cleanup
    Get-Job | Remove-Job -Force -ErrorAction SilentlyContinue
}

# =============================================================================
# COMMAND HANDLERS
# =============================================================================

switch ($Command.ToLower()) {
    "-h" { 
        Write-Host "`nani-tui v$script:VERSION - Anime TUI for Windows`n"
        Write-Host "Usage: ani-tui"
        Write-Host "`nControls:"
        Write-Host "  Type        Search anime"
        Write-Host "  Up/Down     Navigate"
        Write-Host "  Enter       Select/Play"
        Write-Host "  Ctrl-D      Delete from history"
        Write-Host "  Esc         Back/Quit"
        Write-Host "`nDependencies: scoop install fzf chafa ani-cli mpv`n"
    }
    "--help" { & $MyInvocation.MyCommand.Path -h }
    "-v" { Write-Host "ani-tui $script:VERSION" }
    "--version" { & $MyInvocation.MyCommand.Path -v }
    default { Start-TUI }
}
