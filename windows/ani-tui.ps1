<#
.SYNOPSIS
    ani-tui v5.3 - Enhanced Anime TUI for Windows
    
.DESCRIPTION
    Terminal-based anime browser with fzf interface and streaming.
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

$script:VERSION = "5.3.0"

# Directories
$script:DATA_DIR = Join-Path $env:USERPROFILE ".ani-tui"
$script:CACHE_DIR = Join-Path $script:DATA_DIR "cache"
$script:IMAGES_DIR = Join-Path $script:CACHE_DIR "images"
$script:HISTORY_FILE = Join-Path $script:DATA_DIR "history.json"

# AllAnime API
$script:ALLANIME_API = "https://api.allanime.day"
$script:ALLANIME_REFR = "https://allmanga.to"
$script:USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/121.0"

# fzf colors (Catppuccin Mocha)
$script:FZF_COLORS = "fg:#cdd6f4,bg:#1e1e2e,hl:#f9e2af,fg+:#cdd6f4,bg+:#313244,hl+:#f9e2af,info:#94e2d5,prompt:#f5c2e7,pointer:#f5e0dc,marker:#a6e3a1,spinner:#f5e0dc,header:#89b4fa,border:#6c7086"

# =============================================================================
# INITIALIZATION
# =============================================================================

function Initialize-App {
    @($script:DATA_DIR, $script:CACHE_DIR, $script:IMAGES_DIR) | ForEach-Object {
        if (-not (Test-Path $_)) { New-Item -ItemType Directory -Path $_ -Force | Out-Null }
    }
    if (-not (Test-Path $script:HISTORY_FILE)) {
        "[]" | Out-File -FilePath $script:HISTORY_FILE -Encoding UTF8 -NoNewline
    }
}

# =============================================================================
# API FUNCTIONS
# =============================================================================

function Search-Anime {
    param([string]$Query)
    
    if ([string]::IsNullOrWhiteSpace($Query) -or $Query.Length -lt 2) { return @() }
    
    $gql = 'query($search:SearchInput$limit:Int$page:Int$translationType:VaildTranslationTypeEnumType$countryOrigin:VaildCountryOriginEnumType){shows(search:$search limit:$limit page:$page translationType:$translationType countryOrigin:$countryOrigin){edges{_id name availableEpisodes __typename}}}'
    
    $variables = @{
        search = @{ allowAdult = $false; allowUnknown = $false; query = $Query }
        limit = 30; page = 1; translationType = "sub"; countryOrigin = "ALL"
    } | ConvertTo-Json -Compress
    
    try {
        $uri = "$script:ALLANIME_API/api?variables=$([uri]::EscapeDataString($variables))&query=$([uri]::EscapeDataString($gql))"
        $response = Invoke-RestMethod -Uri $uri -Headers @{ "User-Agent" = $script:USER_AGENT; "Referer" = $script:ALLANIME_REFR } -TimeoutSec 10
        
        $results = @()
        foreach ($show in $response.data.shows.edges) {
            $eps = $show.availableEpisodes.sub
            if ($eps -and $eps -gt 0) {
                $results += "$($show._id)`t$($show.name) ($eps eps)"
            }
        }
        return $results
    }
    catch { return @() }
}

function Get-EpisodeList {
    param([string]$ShowId)
    
    $gql = 'query($showId:String!){show(_id:$showId){_id availableEpisodesDetail}}'
    $variables = @{ showId = $ShowId } | ConvertTo-Json -Compress
    
    try {
        $uri = "$script:ALLANIME_API/api?variables=$([uri]::EscapeDataString($variables))&query=$([uri]::EscapeDataString($gql))"
        $response = Invoke-RestMethod -Uri $uri -Headers @{ "User-Agent" = $script:USER_AGENT; "Referer" = $script:ALLANIME_REFR } -TimeoutSec 10
        return $response.data.show.availableEpisodesDetail.sub | Sort-Object { [double]$_ }
    }
    catch { return @() }
}

# =============================================================================
# HISTORY FUNCTIONS
# =============================================================================

function Get-History {
    try {
        $content = Get-Content $script:HISTORY_FILE -Raw -ErrorAction Stop
        if ([string]::IsNullOrWhiteSpace($content)) { return @() }
        $h = $content | ConvertFrom-Json
        if ($null -eq $h) { return @() }
        if ($h -isnot [array]) { return @($h) }
        return $h
    }
    catch { return @() }
}

function Save-History {
    param([string]$Title, [int]$Episode)
    
    $ts = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $history = @(Get-History)
    
    $found = $false
    for ($i = 0; $i -lt $history.Count; $i++) {
        if ($history[$i].title -eq $Title) {
            $history[$i].last_episode = $Episode
            $history[$i].last_watched = $ts
            $found = $true
            break
        }
    }
    
    if (-not $found) {
        $history += [PSCustomObject]@{ title = $Title; last_episode = $Episode; last_watched = $ts }
    }
    
    $history | ConvertTo-Json -Depth 10 | Out-File -FilePath $script:HISTORY_FILE -Encoding UTF8
}

function Delete-FromHistory {
    param([string]$Title)
    $history = @(Get-History) | Where-Object { $_.title -ne $Title }
    if ($null -eq $history) { $history = @() }
    @($history) | ConvertTo-Json -Depth 10 | Out-File -FilePath $script:HISTORY_FILE -Encoding UTF8
}

# =============================================================================
# MAIN TUI - Simplified two-step approach (no reload flickering)
# =============================================================================

function Start-MainLoop {
    Initialize-App
    
    if (-not (Get-Command "fzf" -ErrorAction SilentlyContinue)) {
        Write-Host "fzf not found. Install: scoop install fzf" -ForegroundColor Red
        exit 1
    }
    
    while ($true) {
        Clear-Host
        
        # Build menu items
        $items = @()
        $history = Get-History
        
        # Add history items
        $history | Select-Object -First 10 | ForEach-Object {
            $items += "HIST`t[$($_.last_episode)] $($_.title)"
        }
        
        # Add search option at top
        $items = @("SEARCH`t[Type to search for new anime...]") + $items
        
        # Show main menu
        $header = "ani-tui v$script:VERSION | Enter=Select | Esc=Quit"
        $selected = $items | fzf --ansi --height=100% --layout=reverse --border=rounded --padding=1 `
            --delimiter="`t" --with-nth=2 --header="$header" --header-first `
            --prompt="Select: " --pointer=">" --color="$script:FZF_COLORS"
        
        if ([string]::IsNullOrWhiteSpace($selected)) { break }
        
        # Handle selection
        if ($selected -match "^SEARCH") {
            # Search mode
            $query = ""
            Write-Host ""
            Write-Host "  Enter anime name to search:" -ForegroundColor Cyan
            $query = Read-Host "  Search"
            
            if ([string]::IsNullOrWhiteSpace($query)) { continue }
            
            Write-Host "  Searching..." -ForegroundColor Yellow
            $results = Search-Anime -Query $query
            
            if ($results.Count -eq 0) {
                Write-Host "  No results found for: $query" -ForegroundColor Red
                Start-Sleep -Seconds 2
                continue
            }
            
            # Show search results
            $resultHeader = "Search: $query | Enter=Select | Esc=Back"
            $selectedResult = $results | fzf --ansi --height=100% --layout=reverse --border=rounded --padding=1 `
                --delimiter="`t" --with-nth=2 --header="$resultHeader" --header-first `
                --prompt="Select: " --pointer=">" --color="$script:FZF_COLORS"
            
            if ([string]::IsNullOrWhiteSpace($selectedResult)) { continue }
            
            $parts = $selectedResult -split "`t", 2
            $showId = $parts[0]
            $title = ($parts[1] -replace '\s*\(\d+\s+eps\)$', '').Trim()
        }
        else {
            # History item selected
            $parts = $selected -split "`t", 2
            $title = ($parts[1] -replace '^\[\d+\]\s*', '').Trim()
            
            Write-Host ""
            Write-Host "  Searching: $title" -ForegroundColor Cyan
            $results = Search-Anime -Query $title
            
            if ($results.Count -eq 0) {
                Write-Host "  Could not find anime" -ForegroundColor Red
                Start-Sleep -Seconds 2
                continue
            }
            
            $showId = ($results[0] -split "`t")[0]
        }
        
        # Get episodes
        Write-Host "  Loading episodes..." -ForegroundColor Yellow
        $episodes = Get-EpisodeList -ShowId $showId
        
        if ($episodes.Count -eq 0) {
            Write-Host "  No episodes available" -ForegroundColor Red
            Start-Sleep -Seconds 2
            continue
        }
        
        # Get last watched
        $lastEp = 0
        $history | ForEach-Object { if ($_.title -eq $title) { $lastEp = [int]$_.last_episode } }
        
        # Format episodes
        $epItems = @()
        foreach ($ep in $episodes) {
            $epNum = [int]$ep
            if ($epNum -eq $lastEp) { $epItems += "$ep  << Last watched" }
            elseif ($epNum -eq ($lastEp + 1)) { $epItems += "$ep  >> Continue" }
            else { $epItems += "$ep" }
        }
        
        # Episode selection
        $epHeader = "$title | Enter=Play | Esc=Back"
        $epChoice = $epItems | fzf --ansi --height=100% --layout=reverse --border=rounded --padding=1 `
            --header="$epHeader" --header-first --prompt="Episode: " --pointer=">" --no-info `
            --color="fg:#cdd6f4,bg:#1e1e2e,hl:#a6e3a1,fg+:#cdd6f4,bg+:#313244,hl+:#a6e3a1,prompt:#94e2d5,pointer:#f5e0dc,header:#cba6f7,border:#6c7086"
        
        if ([string]::IsNullOrWhiteSpace($epChoice)) { continue }
        
        $episode = [int](($epChoice -split '\s+')[0])
        
        # Save to history
        Save-History -Title $title -Episode $episode
        
        # Play
        Clear-Host
        Write-Host ""
        Write-Host "  > Now Playing: $title - Episode $episode" -ForegroundColor Green
        Write-Host ""
        
        if (Get-Command "ani-cli" -ErrorAction SilentlyContinue) {
            & ani-cli -S 1 -e $episode "$title"
        }
        else {
            Write-Host "  ani-cli not found." -ForegroundColor Yellow
            Write-Host "  Install: scoop bucket add extras; scoop install ani-cli mpv" -ForegroundColor Gray
            Write-Host ""
            Write-Host "  Episode recorded in history." -ForegroundColor Cyan
            Write-Host ""
            Read-Host "  Press Enter"
        }
    }
}

# =============================================================================
# COMMAND HANDLERS
# =============================================================================

switch ($Command.ToLower()) {
    "--search" {
        # Internal search command (for future use)
        $q = $Arguments -join " "
        Search-Anime -Query $q | ForEach-Object { Write-Output $_ }
    }
    "--history" {
        Initialize-App
        Get-History | Select-Object -First 10 | ForEach-Object {
            Write-Output "HIST`t[$($_.last_episode)] $($_.title)"
        }
    }
    "--delete" {
        Initialize-App
        $t = ($Arguments -join " ") -replace '^\[\d+\]\s*', ''
        Delete-FromHistory -Title $t
    }
    "-h" { 
        Write-Host "`nani-tui v$script:VERSION`n"
        Write-Host "Usage: ani-tui"
        Write-Host "`nControls: Up/Down, Enter, Esc"
        Write-Host "Install deps: scoop install fzf ani-cli mpv`n"
    }
    "--help" { & $PSCommandPath -h }
    "-v" { Write-Host "ani-tui $script:VERSION" }
    "--version" { Write-Host "ani-tui $script:VERSION" }
    default { Start-MainLoop }
}
