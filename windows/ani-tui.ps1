<#
.SYNOPSIS
    ani-tui v5.5 - Anime TUI for Windows (Stable)
.DESCRIPTION
    Reliable Windows implementation with smooth UI.
    Press Enter after typing to search (no flickering).
#>

param([string]$Command = "", [string[]]$Arguments)

# Config
$VERSION = "5.5.0"
$DATA_DIR = "$env:USERPROFILE\.ani-tui"
$CACHE_DIR = "$DATA_DIR\cache"
$IMAGES_DIR = "$CACHE_DIR\images"
$HISTORY_FILE = "$DATA_DIR\history.json"
$ALLANIME_API = "https://api.allanime.day"

# Colors
$FZF_MAIN = "fg:#cdd6f4,bg:#1e1e2e,hl:#f9e2af,fg+:#cdd6f4,bg+:#313244,hl+:#f9e2af,info:#94e2d5,prompt:#f5c2e7,pointer:#f5e0dc,marker:#a6e3a1,header:#89b4fa,border:#6c7086"
$FZF_EP = "fg:#cdd6f4,bg:#1e1e2e,hl:#a6e3a1,fg+:#cdd6f4,bg+:#313244,hl+:#a6e3a1,prompt:#94e2d5,pointer:#f5e0dc,header:#cba6f7,border:#6c7086"

# =============================================================================
# INIT
# =============================================================================
function Init {
    $dirs = @($DATA_DIR, $CACHE_DIR, $IMAGES_DIR)
    foreach ($d in $dirs) {
        if (!(Test-Path $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null }
    }
    if (!(Test-Path $HISTORY_FILE)) { "[]" | Out-File $HISTORY_FILE -Encoding UTF8 }
}

# =============================================================================
# HISTORY (same format as macOS)
# =============================================================================
function Get-WatchHistory {
    try {
        $c = Get-Content $HISTORY_FILE -Raw -ErrorAction Stop
        if (!$c -or $c -eq "[]") { return @() }
        $h = $c | ConvertFrom-Json
        if ($h -isnot [array]) { return @($h) }
        return $h
    } catch { return @() }
}

function Save-WatchHistory($title, $ep) {
    $ts = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $h = @(Get-WatchHistory)
    $found = $false
    for ($i = 0; $i -lt $h.Count; $i++) {
        if ($h[$i].title -eq $title) {
            $h[$i].last_episode = $ep
            $h[$i].last_watched = $ts
            $found = $true
            break
        }
    }
    if (!$found) {
        $h += [PSCustomObject]@{title=$title; last_episode=$ep; last_watched=$ts}
    }
    $h | ConvertTo-Json -Depth 10 | Out-File $HISTORY_FILE -Encoding UTF8
}

function Remove-WatchHistory($title) {
    $h = @(Get-WatchHistory) | Where-Object { $_.title -ne $title }
    @($h) | ConvertTo-Json -Depth 10 | Out-File $HISTORY_FILE -Encoding UTF8
}

# =============================================================================
# ALLANIME API
# =============================================================================
function Search-Anime($query) {
    if (!$query -or $query.Length -lt 2) { return @() }
    
    $gql = 'query($search:SearchInput$limit:Int$page:Int$translationType:VaildTranslationTypeEnumType$countryOrigin:VaildCountryOriginEnumType){shows(search:$search limit:$limit page:$page translationType:$translationType countryOrigin:$countryOrigin){edges{_id name availableEpisodes}}}'
    $vars = (@{search=@{allowAdult=$false;allowUnknown=$false;query=$query};limit=30;page=1;translationType="sub";countryOrigin="ALL"} | ConvertTo-Json -Compress)
    
    try {
        $uri = "$ALLANIME_API/api?variables=$([uri]::EscapeDataString($vars))&query=$([uri]::EscapeDataString($gql))"
        $r = Invoke-RestMethod $uri -Headers @{"User-Agent"="Mozilla/5.0";"Referer"="https://allmanga.to"} -TimeoutSec 15
        $results = @()
        foreach ($s in $r.data.shows.edges) {
            if ($s.availableEpisodes.sub -gt 0) {
                $results += [PSCustomObject]@{id=$s._id; name=$s.name; eps=$s.availableEpisodes.sub}
            }
        }
        return $results
    } catch { return @() }
}

function Get-Episodes($showId) {
    $gql = 'query($showId:String!){show(_id:$showId){availableEpisodesDetail}}'
    $vars = (@{showId=$showId} | ConvertTo-Json -Compress)
    
    try {
        $uri = "$ALLANIME_API/api?variables=$([uri]::EscapeDataString($vars))&query=$([uri]::EscapeDataString($gql))"
        $r = Invoke-RestMethod $uri -Headers @{"User-Agent"="Mozilla/5.0";"Referer"="https://allmanga.to"} -TimeoutSec 15
        return $r.data.show.availableEpisodesDetail.sub | Sort-Object {[double]$_}
    } catch { return @() }
}

# =============================================================================
# TUI COMPONENTS
# =============================================================================

function Show-MainMenu {
    $history = Get-WatchHistory
    
    # Build menu
    $items = @()
    $items += ">> SEARCH FOR ANIME <<"
    $history | Select-Object -First 15 | ForEach-Object {
        $items += "[Ep $($_.last_episode)] $($_.title)"
    }
    
    $header = "ani-tui v$VERSION | Enter=Select | D=Delete | Esc=Quit"
    
    $selected = $items | fzf --ansi --height=100% --layout=reverse --border=rounded `
        --header="$header" --header-first --prompt="> " --pointer=">" `
        --bind="d:execute-silent(echo {})"+""  `
        --color="$FZF_MAIN" --no-info 2>$null
    
    return $selected
}

function Show-SearchPrompt {
    Clear-Host
    Write-Host ""
    Write-Host "  ========== ANIME SEARCH ==========" -ForegroundColor Cyan
    Write-Host ""
    $query = Read-Host "  Enter anime name"
    return $query
}

function Show-SearchResults($results) {
    if ($results.Count -eq 0) { return $null }
    
    $items = $results | ForEach-Object { "$($_.id)`t$($_.name) ($($_.eps) eps)" }
    
    $header = "Search Results | Enter=Select | Esc=Back"
    $selected = $items | fzf --ansi --height=100% --layout=reverse --border=rounded `
        --delimiter="`t" --with-nth=2 `
        --header="$header" --header-first --prompt="Select: " --pointer=">" `
        --color="$FZF_MAIN" --no-info 2>$null
    
    return $selected
}

function Show-EpisodeList($title, $episodes, $lastEp) {
    $items = foreach ($ep in $episodes) {
        $n = [int]$ep
        if ($n -eq $lastEp) { "$ep  << Last watched" }
        elseif ($n -eq $lastEp + 1) { "$ep  >> Continue here" }
        else { "$ep" }
    }
    
    $header = "$title | Enter=Play | Esc=Back"
    $selected = $items | fzf --ansi --height=100% --layout=reverse --border=rounded `
        --header="$header" --header-first --prompt="Episode: " --pointer=">" `
        --color="$FZF_EP" --no-info 2>$null
    
    return $selected
}

# =============================================================================
# MAIN LOOP
# =============================================================================
function Start-App {
    Init
    
    # Check fzf
    if (!(Get-Command "fzf" -ErrorAction SilentlyContinue)) {
        Write-Host "`nfzf not found!" -ForegroundColor Red
        Write-Host "Install: scoop install fzf`n" -ForegroundColor Yellow
        exit 1
    }
    
    while ($true) {
        Clear-Host
        $selection = Show-MainMenu
        
        if (!$selection) { break }
        
        $showId = $null
        $title = $null
        
        if ($selection -eq ">> SEARCH FOR ANIME <<") {
            # Search mode
            $query = Show-SearchPrompt
            if (!$query) { continue }
            
            Write-Host "`n  Searching for: $query" -ForegroundColor Yellow
            $results = Search-Anime $query
            
            if ($results.Count -eq 0) {
                Write-Host "  No results found." -ForegroundColor Red
                Start-Sleep 2
                continue
            }
            
            Write-Host "  Found $($results.Count) results." -ForegroundColor Green
            Start-Sleep 1
            
            $selected = Show-SearchResults $results
            if (!$selected) { continue }
            
            $parts = $selected -split "`t"
            $showId = $parts[0]
            $title = ($parts[1] -replace '\s*\(\d+\s+eps\)$','').Trim()
        }
        else {
            # History item
            $title = ($selection -replace '^\[Ep \d+\]\s*','').Trim()
            
            # Check for delete action (D key)
            # Note: fzf doesn't easily support custom key actions in basic mode
            
            Write-Host "`n  Loading: $title" -ForegroundColor Yellow
            $results = Search-Anime $title
            
            if ($results.Count -eq 0) {
                Write-Host "  Could not find anime." -ForegroundColor Red
                Start-Sleep 2
                continue
            }
            
            $showId = $results[0].id
        }
        
        if (!$showId) { continue }
        
        # Get episodes
        Write-Host "  Fetching episodes..." -ForegroundColor Yellow
        $episodes = @(Get-Episodes $showId)
        
        if ($episodes.Count -eq 0) {
            Write-Host "  No episodes available." -ForegroundColor Red
            Start-Sleep 2
            continue
        }
        
        # Get last watched
        $lastEp = 0
        Get-WatchHistory | ForEach-Object {
            if ($_.title -eq $title) { $lastEp = [int]$_.last_episode }
        }
        
        # Show episodes
        $epChoice = Show-EpisodeList $title $episodes $lastEp
        if (!$epChoice) { continue }
        
        $episode = [int](($epChoice -split '\s+')[0])
        
        # Save history
        Save-WatchHistory $title $episode
        
        # Play
        Clear-Host
        Write-Host ""
        Write-Host "  ========================================" -ForegroundColor Green
        Write-Host "   Now Playing: $title" -ForegroundColor White
        Write-Host "   Episode: $episode" -ForegroundColor Cyan
        Write-Host "  ========================================" -ForegroundColor Green
        Write-Host ""
        
        if (Get-Command "ani-cli" -ErrorAction SilentlyContinue) {
            & ani-cli -S 1 -e $episode "$title"
        }
        else {
            Write-Host "  ani-cli not found." -ForegroundColor Yellow
            Write-Host "  Install: scoop bucket add extras" -ForegroundColor Gray
            Write-Host "           scoop install ani-cli mpv" -ForegroundColor Gray
            Write-Host ""
            Write-Host "  Episode saved to history." -ForegroundColor Cyan
            Write-Host ""
            Read-Host "  Press Enter to continue"
        }
    }
    
    Clear-Host
    Write-Host "`n  Goodbye!`n" -ForegroundColor Cyan
}

# =============================================================================
# ENTRY
# =============================================================================
switch ($Command.ToLower()) {
    "-h" { 
        Write-Host @"

ani-tui v$VERSION - Anime TUI for Windows

USAGE: ani-tui

CONTROLS:
  Arrow keys  Navigate
  Enter       Select / Play
  Esc         Back / Quit

INSTALL DEPENDENCIES:
  scoop install fzf
  scoop bucket add extras
  scoop install ani-cli mpv

"@
    }
    "--help" { & $PSCommandPath -h }
    "-v" { Write-Host "ani-tui $VERSION" }
    "--version" { & $PSCommandPath -v }
    default { Start-App }
}
