<#
.SYNOPSIS
    ani-tui v6.4 for Windows - Fixed Preview and Playback
.DESCRIPTION
    - Forces regeneration of helper scripts for preview fix
    - Uses Git Bash for ani-cli playback (required by ani-cli)
    - High quality image previews
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
$script:VERSION = "6.4.0"
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
# SETUP
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
    
    # Always regenerate helper scripts to ensure latest version
    Create-HelperScripts
}

function Create-HelperScripts {
    # ==========================================================================
    # SEARCH HELPER
    # ==========================================================================
    $searchScript = @'
[CmdletBinding()]
param([Parameter(ValueFromRemainingArguments)][string[]]$QueryArgs)
$ErrorActionPreference = "SilentlyContinue"
$q = ($QueryArgs -join " ").Trim()
$HIST = "$env:USERPROFILE\.ani-tui\history.json"

# If query is empty or too short, show history
if (!$q -or $q.Length -lt 2) {
    try {
        $h = Get-Content $HIST -Raw | ConvertFrom-Json
        $h | Select-Object -First 10 | ForEach-Object {
            "HIST`t[$($_.last_episode)] $($_.title)"
        }
    } catch {}
    exit
}

# Search API
$gql = 'query($search:SearchInput$limit:Int$page:Int$translationType:VaildTranslationTypeEnumType$countryOrigin:VaildCountryOriginEnumType){shows(search:$search limit:$limit page:$page translationType:$translationType countryOrigin:$countryOrigin){edges{_id name availableEpisodes}}}'
$vars = @{search=@{allowAdult=$false;allowUnknown=$false;query=$q};limit=30;page=1;translationType="sub";countryOrigin="ALL"} | ConvertTo-Json -Compress

try {
    $url = "https://api.allanime.day/api?variables=$([uri]::EscapeDataString($vars))&query=$([uri]::EscapeDataString($gql))"
    $r = Invoke-RestMethod $url -Headers @{"User-Agent"="Mozilla/5.0";"Referer"="https://allmanga.to"} -TimeoutSec 10
    $r.data.shows.edges | Where-Object { $_.availableEpisodes.sub -gt 0 } | ForEach-Object {
        "$($_._id)`t$($_.name) ($($_.availableEpisodes.sub) eps)"
    }
} catch {}
'@
    $searchScript | Out-File "$script:SCRIPTS\search.ps1" -Encoding UTF8
    
    $searchCmd = @'
@echo off
powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0search.ps1" %*
'@
    $searchCmd | Out-File "$script:SCRIPTS\search.cmd" -Encoding ASCII

    # ==========================================================================
    # HISTORY HELPER
    # ==========================================================================
    $historyScript = @'
[CmdletBinding()]
param()
$ErrorActionPreference = "SilentlyContinue"
$HIST = "$env:USERPROFILE\.ani-tui\history.json"
try {
    $h = Get-Content $HIST -Raw | ConvertFrom-Json
    $h | Select-Object -First 10 | ForEach-Object {
        "HIST`t[$($_.last_episode)] $($_.title)"
    }
} catch {}
'@
    $historyScript | Out-File "$script:SCRIPTS\history.ps1" -Encoding UTF8
    
    $historyCmd = @'
@echo off
powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0history.ps1"
'@
    $historyCmd | Out-File "$script:SCRIPTS\history.cmd" -Encoding ASCII

    # ==========================================================================
    # DELETE HELPER
    # ==========================================================================
    $deleteScript = @'
[CmdletBinding()]
param([Parameter(ValueFromRemainingArguments)][string[]]$InputArgs)
$ErrorActionPreference = "SilentlyContinue"
$inputText = ($InputArgs -join " ").Trim()
$HIST = "$env:USERPROFILE\.ani-tui\history.json"

# Extract title (remove [xx] prefix)
$title = $inputText -replace '^\[\d+\]\s*', ''
if (!$title) { exit }

try {
    $h = @((Get-Content $HIST -Raw | ConvertFrom-Json) | Where-Object { $_.title -ne $title })
    if ($h.Count -eq 0) { "[]" } else { $h | ConvertTo-Json -Depth 10 }
    | Out-File $HIST -Encoding UTF8
} catch {}
'@
    $deleteScript | Out-File "$script:SCRIPTS\delete.ps1" -Encoding UTF8
    
    $deleteCmd = @'
@echo off
powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0delete.ps1" %*
'@
    $deleteCmd | Out-File "$script:SCRIPTS\delete.cmd" -Encoding ASCII

    # ==========================================================================
    # PREVIEW HELPER - Image + Anime Info (hybrid approach)
    # ==========================================================================
    $previewScript = @'
[CmdletBinding()]
param([Parameter(ValueFromRemainingArguments)][string[]]$InputArgs)
$ErrorActionPreference = "SilentlyContinue"
$inputText = ($InputArgs -join " ").Trim()
if (!$inputText) { exit }

$CACHE = "$env:USERPROFILE\.ani-tui\cache"
$IMAGES = "$CACHE\images"
$INFO_CACHE = "$CACHE\info"
if (!(Test-Path $IMAGES)) { New-Item -ItemType Directory -Path $IMAGES -Force | Out-Null }
if (!(Test-Path $INFO_CACHE)) { New-Item -ItemType Directory -Path $INFO_CACHE -Force | Out-Null }

# Clean title
$name = $inputText
$name = $name -replace '^HIST\s*', ''
$name = $name -replace '^\[\d+\]\s*', ''
$name = $name -replace '\s*\(\d+\s+eps\)\s*$', ''
$name = $name -replace '\s*\[[A-Z]+\]\s*$', ''
$name = $name.Trim()

if (!$name) { exit }

# Generate hash for cache
$bytes = [System.Text.Encoding]::UTF8.GetBytes($name)
$md5 = [System.Security.Cryptography.MD5]::Create()
$hash = [System.BitConverter]::ToString($md5.ComputeHash($bytes)).Replace("-", "").Substring(0, 12).ToLower()
$imgPath = "$IMAGES\$hash.jpg"
$infoPath = "$INFO_CACHE\$hash.json"

# ============================================================================
# PART 1: IMAGE PREVIEW (using same settings as macOS version)
# ============================================================================
$hasChafa = Get-Command chafa -ErrorAction SilentlyContinue

if ($hasChafa) {
    # Fetch image if not cached
    if (!(Test-Path $imgPath)) {
        try {
            $escapedName = $name -replace '"', '\"'
            $imgQuery = @{
                query = "query{Page(perPage:1){media(search:`"$escapedName`",type:ANIME){coverImage{extraLarge large}}}}"
            } | ConvertTo-Json -Compress
            
            $imgResponse = Invoke-RestMethod "https://graphql.anilist.co" -Method Post -ContentType "application/json" -Body $imgQuery -TimeoutSec 5
            $coverUrl = $imgResponse.data.Page.media[0].coverImage.extraLarge
            if (!$coverUrl) { $coverUrl = $imgResponse.data.Page.media[0].coverImage.large }
            
            if ($coverUrl) {
                Invoke-WebRequest $coverUrl -OutFile $imgPath -TimeoutSec 8 | Out-Null
            }
        } catch {}
    }
    
    # Display image with SAME settings as macOS version
    if (Test-Path $imgPath) {
        Write-Host ""
        # Using macOS-matching settings: --size=70x45 --symbols=all --colors=256
        & chafa --size=60x35 --symbols=all --colors=256 $imgPath 2>$null
        Write-Host ""
    }
}

# ============================================================================
# PART 2: COMPACT ANIME INFO
# ============================================================================
$info = $null
if (Test-Path $infoPath) {
    try { $info = Get-Content $infoPath -Raw | ConvertFrom-Json } catch {}
}

if (!$info) {
    try {
        $escapedName = $name -replace '"', '\"'
        $query = @{
            query = "query{Page(perPage:1){media(search:`"$escapedName`",type:ANIME){title{english romaji native}averageScore status episodes format season seasonYear genres studios(isMain:true){nodes{name}}}}}"
        } | ConvertTo-Json -Compress
        
        $response = Invoke-RestMethod "https://graphql.anilist.co" -Method Post -ContentType "application/json" -Body $query -TimeoutSec 5
        $info = $response.data.Page.media[0]
        
        if ($info) {
            $info | ConvertTo-Json -Depth 10 | Out-File $infoPath -Encoding UTF8
        }
    } catch {}
}

# Show title if no image was displayed
if (!$hasChafa -or !(Test-Path $imgPath)) {
    Write-Host ""
    Write-Host "  $name" -ForegroundColor Cyan
    Write-Host "  ────────────────────────────────" -ForegroundColor DarkGray
}

if ($info) {
    # Title (if image shown, show compact title)
    $title = if ($info.title.english) { $info.title.english } else { $info.title.romaji }
    
    if ($hasChafa -and (Test-Path $imgPath)) {
        Write-Host "  $title" -ForegroundColor Cyan
    }
    
    # Score | Status | Episodes (one line)
    $score = if ($info.averageScore) { "$($info.averageScore)%" } else { "N/A" }
    $status = if ($info.status) { $info.status } else { "?" }
    $eps = if ($info.episodes) { "$($info.episodes) eps" } else { "? eps" }
    $scoreColor = if ($info.averageScore -ge 80) { "Green" } elseif ($info.averageScore -ge 60) { "Yellow" } else { "Gray" }
    
    Write-Host "  " -NoNewline
    Write-Host "★ $score" -NoNewline -ForegroundColor $scoreColor
    Write-Host " | " -NoNewline -ForegroundColor DarkGray
    Write-Host "$status" -NoNewline -ForegroundColor Magenta
    Write-Host " | " -NoNewline -ForegroundColor DarkGray
    Write-Host "$eps" -ForegroundColor White
    
    # Format | Season | Studio (one line)
    $format = if ($info.format) { $info.format } else { "" }
    $season = if ($info.season -and $info.seasonYear) { "$($info.season) $($info.seasonYear)" } else { "" }
    $studio = if ($info.studios.nodes -and $info.studios.nodes.Count -gt 0) { $info.studios.nodes[0].name } else { "" }
    
    $parts = @($format, $season, $studio) | Where-Object { $_ }
    if ($parts.Count -gt 0) {
        Write-Host "  $($parts -join ' • ')" -ForegroundColor Gray
    }
    
    # Genres (one line)
    if ($info.genres -and $info.genres.Count -gt 0) {
        $genreList = ($info.genres | Select-Object -First 4) -join " • "
        Write-Host "  $genreList" -ForegroundColor DarkYellow
    }
} else {
    if (!$hasChafa) {
        Write-Host "  [Install chafa: scoop install chafa]" -ForegroundColor DarkGray
    }
}

Write-Host ""
'@
    $previewScript | Out-File "$script:SCRIPTS\preview.ps1" -Encoding UTF8
    
    $previewCmd = @'
@echo off
powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0preview.ps1" %*
'@
    $previewCmd | Out-File "$script:SCRIPTS\preview.cmd" -Encoding ASCII
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
# API FUNCTIONS
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
# FIND GIT BASH
# =============================================================================
function Find-GitBash {
    # Check common Git Bash locations
    $paths = @(
        "$env:ProgramFiles\Git\bin\bash.exe",
        "$env:ProgramFiles(x86)\Git\bin\bash.exe",
        "$env:USERPROFILE\scoop\apps\git\current\bin\bash.exe",
        "C:\Program Files\Git\bin\bash.exe",
        "C:\Program Files (x86)\Git\bin\bash.exe"
    )
    
    # Also check GIT_INSTALL_ROOT environment variable
    if ($env:GIT_INSTALL_ROOT) {
        $paths = @("$env:GIT_INSTALL_ROOT\bin\bash.exe") + $paths
    }
    
    foreach ($path in $paths) {
        if (Test-Path $path) {
            return $path
        }
    }
    
    # Try to find via where command
    $gitBash = Get-Command bash.exe -ErrorAction SilentlyContinue | Where-Object { $_.Source -notlike "*System32*" -and $_.Source -notlike "*WSL*" }
    if ($gitBash) {
        return $gitBash.Source
    }
    
    return $null
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
    
    while ($true) {
        # Get initial items (history)
        $items = @()
        Get-AnimeHistory | Select-Object -First 10 | ForEach-Object {
            $items += "HIST`t[$($_.last_episode)] $($_.title)"
        }
        
        # Use helper scripts for fzf callbacks
        $searchCmd = "`"$script:SCRIPTS\search.cmd`" {q}"
        $historyCmd = "`"$script:SCRIPTS\history.cmd`""
        $deleteCmd = "`"$script:SCRIPTS\delete.cmd`" {2}"
        $previewCmd = "`"$script:SCRIPTS\preview.cmd`" {2}"
        
        $header = "ani-tui v$script:VERSION | Type=Search | Enter=Select | Ctrl-D=Delete | Esc=Quit"
        
        # Run fzf
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
            # From history
            $title = ($parts[1] -replace '^\[\d+\]\s*', '').Trim()
            Write-Host ""
            Write-Host "  Searching: $title" -ForegroundColor Cyan
            $results = @(Search-Anime $title)
            if ($results.Count -gt 0) {
                $showId = $results[0].id
            }
        } else {
            # From search
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
        
        # Get last watched
        $lastEp = 0
        Get-AnimeHistory | ForEach-Object {
            if ($_.title -eq $title) { $lastEp = [int]$_.last_episode }
        }
        
        # Format episodes
        $epItems = foreach ($ep in $episodes) {
            $epNum = [int]$ep
            if ($epNum -eq $lastEp) { "$ep  << Last watched" }
            elseif ($epNum -eq ($lastEp + 1)) { "$ep  >> Continue" }
            else { "$ep" }
        }
        
        # Episode selection
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
        
        $episode = [int](($epSelection -split '\s+')[0])
        
        # Save history
        Save-AnimeHistory $title $episode
        
        # Play with ani-cli
        Clear-Host
        Write-Host ""
        Write-Host "  >> Now Playing: $title - Episode $episode" -ForegroundColor Green
        Write-Host ""
        
        # Check for ani-cli
        $aniCli = Get-Command ani-cli -ErrorAction SilentlyContinue
        if (!$aniCli) {
            Write-Host "  ani-cli not found." -ForegroundColor Yellow
            Write-Host ""
            Write-Host "  Install streaming support:" -ForegroundColor Gray
            Write-Host "    scoop bucket add extras" -ForegroundColor DarkGray
            Write-Host "    scoop install ani-cli mpv" -ForegroundColor DarkGray
            Write-Host ""
            Write-Host "  Episode saved to history." -ForegroundColor Cyan
            Write-Host ""
            Read-Host "  Press Enter to continue"
            continue
        }
        
        # Find Git Bash - required for ani-cli on Windows
        $gitBash = Find-GitBash
        
        if ($gitBash) {
            # Run ani-cli through Git Bash (required for proper operation)
            Write-Host "  Using Git Bash for ani-cli..." -ForegroundColor DarkGray
            $escapedTitle = $title -replace "'", "'\''"
            $bashCmd = "ani-cli -S 1 -e $episode '$escapedTitle'"
            
            Start-Process -FilePath $gitBash -ArgumentList "-c", "`"$bashCmd`"" -Wait -NoNewWindow
        } else {
            # Fallback: try direct execution (may have issues)
            Write-Host "  Git Bash not found, trying direct execution..." -ForegroundColor Yellow
            Write-Host "  (For best results, install Git for Windows: scoop install git)" -ForegroundColor DarkGray
            Write-Host ""
            
            try {
                & ani-cli -S 1 -e $episode $title
            } catch {
                Write-Host "  Error running ani-cli: $_" -ForegroundColor Red
                Write-Host "  ani-cli requires Git Bash on Windows." -ForegroundColor Yellow
                Write-Host "  Install: scoop install git" -ForegroundColor Yellow
            }
        }
        
        Write-Host ""
        Write-Host "  Playback ended." -ForegroundColor Cyan
        Read-Host "  Press Enter to continue"
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
        Write-Host "    scoop install git fzf chafa ani-cli mpv"
        Write-Host ""
    }
    "--help" { & $PSCommandPath -h }
    "-v" { Write-Host "ani-tui $script:VERSION" }
    "--version" { & $PSCommandPath -v }
    default { Start-TUI }
}
