<#
.SYNOPSIS
    ani-tui - Enhanced anime TUI for Windows

.DESCRIPTION
    Terminal-based anime search and dashboard with AniList integration.
    Windows PowerShell implementation with text-based UI.

.EXAMPLE
    .\ani-tui.ps1
    Interactive search mode

.EXAMPLE
    .\ani-tui.ps1 search "naruto"
    Search for anime

.EXAMPLE
    .\ani-tui.ps1 dashboard
    View watched anime dashboard
#>

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Command = "",
    
    [Parameter(Position = 1, ValueFromRemainingArguments)]
    [string[]]$Arguments,
    
    [switch]$Help,
    [switch]$Version
)

# =============================================================================
# CONFIGURATION
# =============================================================================

$script:VERSION = "1.0.0"
$script:SCRIPT_NAME = "ani-tui"

# Directories
$script:DATA_DIR = Join-Path $env:USERPROFILE ".ani-tui"
$script:CACHE_DIR = Join-Path $script:DATA_DIR "cache"
$script:METADATA_CACHE = Join-Path $script:CACHE_DIR "metadata.json"
$script:HISTORY_FILE = Join-Path $script:DATA_DIR "history.json"

# API Configuration
$script:ANILIST_API = "https://graphql.anilist.co"
$script:USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"

# =============================================================================
# UTILITY FUNCTIONS
# =============================================================================

function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] " -ForegroundColor Blue -NoNewline
    Write-Host $Message
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[WARN] " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Error2 {
    param([string]$Message)
    Write-Host "[ERROR] " -ForegroundColor Red -NoNewline
    Write-Host $Message
}

function Write-Success {
    param([string]$Message)
    Write-Host "[OK] " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

# =============================================================================
# DIRECTORY SETUP
# =============================================================================

function Ensure-Directories {
    if (-not (Test-Path $script:DATA_DIR)) {
        New-Item -ItemType Directory -Path $script:DATA_DIR -Force | Out-Null
    }
    if (-not (Test-Path $script:CACHE_DIR)) {
        New-Item -ItemType Directory -Path $script:CACHE_DIR -Force | Out-Null
    }
    if (-not (Test-Path $script:METADATA_CACHE)) {
        "{}" | Out-File -FilePath $script:METADATA_CACHE -Encoding UTF8
    }
    if (-not (Test-Path $script:HISTORY_FILE)) {
        "[]" | Out-File -FilePath $script:HISTORY_FILE -Encoding UTF8
    }
}

# =============================================================================
# ANILIST API FUNCTIONS
# =============================================================================

function Search-AniList {
    param([string]$Query)
    
    $gqlQuery = @"
query (`$search: String) {
    Page(page: 1, perPage: 40) {
        media(search: `$search, type: ANIME, sort: POPULARITY_DESC) {
            id
            title {
                romaji
                english
            }
            coverImage {
                large
            }
            episodes
            status
            averageScore
            description(asHtml: false)
            genres
        }
    }
}
"@

    $body = @{
        query = $gqlQuery
        variables = @{
            search = $Query
        }
    } | ConvertTo-Json -Depth 10

    try {
        $response = Invoke-RestMethod -Uri $script:ANILIST_API `
            -Method Post `
            -Headers @{ "User-Agent" = $script:USER_AGENT } `
            -ContentType "application/json" `
            -Body $body
        return $response
    }
    catch {
        Write-Error2 "Failed to fetch from AniList API: $_"
        return $null
    }
}

function Get-AnimeById {
    param([int]$AnimeId)
    
    $gqlQuery = @"
query (`$id: Int) {
    Media(id: `$id, type: ANIME) {
        id
        title {
            romaji
            english
        }
        coverImage {
            large
        }
        episodes
        status
        averageScore
        description(asHtml: false)
        genres
    }
}
"@

    $body = @{
        query = $gqlQuery
        variables = @{
            id = $AnimeId
        }
    } | ConvertTo-Json -Depth 10

    try {
        $response = Invoke-RestMethod -Uri $script:ANILIST_API `
            -Method Post `
            -Headers @{ "User-Agent" = $script:USER_AGENT } `
            -ContentType "application/json" `
            -Body $body
        return $response.data.Media
    }
    catch {
        return $null
    }
}

# =============================================================================
# CACHING FUNCTIONS
# =============================================================================

function Get-CachedMetadata {
    param([string]$AnimeId)
    
    try {
        $cache = Get-Content $script:METADATA_CACHE -Raw | ConvertFrom-Json
        if ($cache.PSObject.Properties.Name -contains $AnimeId) {
            return $cache.$AnimeId
        }
    }
    catch { }
    return $null
}

function Set-CachedMetadata {
    param(
        [string]$AnimeId,
        [object]$Metadata
    )
    
    try {
        $cache = Get-Content $script:METADATA_CACHE -Raw | ConvertFrom-Json
        $cache | Add-Member -NotePropertyName $AnimeId -NotePropertyValue $Metadata -Force
        $cache | ConvertTo-Json -Depth 10 | Out-File -FilePath $script:METADATA_CACHE -Encoding UTF8
    }
    catch {
        @{ $AnimeId = $Metadata } | ConvertTo-Json -Depth 10 | Out-File -FilePath $script:METADATA_CACHE -Encoding UTF8
    }
}

# =============================================================================
# HISTORY FUNCTIONS
# =============================================================================

function Get-WatchHistory {
    try {
        $history = Get-Content $script:HISTORY_FILE -Raw | ConvertFrom-Json
        return $history
    }
    catch {
        return @()
    }
}

function Add-WatchHistory {
    param(
        [int]$AnimeId,
        [string]$Title,
        [int]$Episode,
        [string]$CoverUrl = ""
    )
    
    $timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $history = Get-WatchHistory
    
    # Convert to array if single object
    if ($history -isnot [array]) {
        $history = @($history)
    }
    
    $existingIndex = -1
    for ($i = 0; $i -lt $history.Count; $i++) {
        if ($history[$i].id -eq $AnimeId) {
            $existingIndex = $i
            break
        }
    }
    
    if ($existingIndex -ge 0) {
        $history[$existingIndex].last_episode = $Episode
        $history[$existingIndex].last_watched = $timestamp
    }
    else {
        $newEntry = @{
            id = $AnimeId
            title = $Title
            last_episode = $Episode
            last_watched = $timestamp
            cover_url = $CoverUrl
        }
        $history += $newEntry
    }
    
    $history | ConvertTo-Json -Depth 10 | Out-File -FilePath $script:HISTORY_FILE -Encoding UTF8
}

function Remove-WatchHistory {
    param([int]$AnimeId)
    
    $history = Get-WatchHistory
    $history = $history | Where-Object { $_.id -ne $AnimeId }
    
    if ($null -eq $history) { $history = @() }
    
    $history | ConvertTo-Json -Depth 10 | Out-File -FilePath $script:HISTORY_FILE -Encoding UTF8
}

# =============================================================================
# FUZZY SEARCH (Simple PowerShell Implementation)
# =============================================================================

function Show-FuzzyMenu {
    param(
        [array]$Items,
        [string]$Header = "Select item:",
        [scriptblock]$DisplayFormatter = { param($item) $item },
        [scriptblock]$OnPreview = $null
    )
    
    if ($Items.Count -eq 0) {
        Write-Warn "No items to display"
        return $null
    }
    
    $filterText = ""
    $selectedIndex = 0
    $filteredItems = $Items
    
    # Clear screen
    Clear-Host
    
    while ($true) {
        # Filter items
        if ($filterText) {
            $filteredItems = $Items | Where-Object { 
                $display = & $DisplayFormatter $_
                $display -like "*$filterText*"
            }
        }
        else {
            $filteredItems = $Items
        }
        
        if ($filteredItems.Count -eq 0) {
            $filteredItems = @()
        }
        
        # Ensure selected index is valid
        if ($selectedIndex -ge $filteredItems.Count) {
            $selectedIndex = [Math]::Max(0, $filteredItems.Count - 1)
        }
        
        # Render
        Clear-Host
        Write-Host ""
        Write-Host "  $Header" -ForegroundColor Cyan
        Write-Host "  Filter: $filterText" -ForegroundColor Yellow
        Write-Host "  (Type to filter, Up/Down to navigate, Enter to select, Esc to cancel)" -ForegroundColor DarkGray
        Write-Host ""
        
        # Show items (max 15)
        $startIndex = [Math]::Max(0, $selectedIndex - 7)
        $endIndex = [Math]::Min($filteredItems.Count, $startIndex + 15)
        
        for ($i = $startIndex; $i -lt $endIndex; $i++) {
            $display = & $DisplayFormatter $filteredItems[$i]
        if ($i -eq $selectedIndex) {
                Write-Host "  > " -ForegroundColor Green -NoNewline
                Write-Host $display -ForegroundColor White
            }
            else {
                Write-Host "    $display" -ForegroundColor Gray
            }
        }
        
        # Show preview if available
        if ($OnPreview -and $filteredItems.Count -gt 0) {
            Write-Host ""
            Write-Host "  -------------------------------------" -ForegroundColor DarkGray
            & $OnPreview $filteredItems[$selectedIndex]
        }
        
        Write-Host ""
        Write-Host "  $($filteredItems.Count) items" -ForegroundColor DarkGray
        
        # Read key
        $key = [Console]::ReadKey($true)
        
        switch ($key.Key) {
            "UpArrow" {
                if ($selectedIndex -gt 0) { $selectedIndex-- }
            }
            "DownArrow" {
                if ($selectedIndex -lt $filteredItems.Count - 1) { $selectedIndex++ }
            }
            "Enter" {
                if ($filteredItems.Count -gt 0) {
                    return $filteredItems[$selectedIndex]
                }
            }
            "Escape" {
                return $null
            }
            "Backspace" {
                if ($filterText.Length -gt 0) {
                    $filterText = $filterText.Substring(0, $filterText.Length - 1)
                }
            }
            default {
                if ($key.KeyChar -match '[\w\s\-]') {
                    $filterText += $key.KeyChar
                }
            }
        }
    }
}

# =============================================================================
# SEARCH MODE
# =============================================================================

function Invoke-SearchMode {
    param([string]$Query = "")
    
    if (-not $Query) {
        $Query = Read-Host "Search anime"
        if (-not $Query) {
            Write-Warn "No search query provided"
            return
        }
    }
    
    Write-Info "Searching for: $Query"
    
    $response = Search-AniList -Query $Query
    
    if (-not $response -or -not $response.data.Page.media) {
        Write-Error2 "No results found for: $Query"
        return
    }
    
    $results = $response.data.Page.media
    
    # Cache all results
    foreach ($media in $results) {
        Set-CachedMetadata -AnimeId $media.id.ToString() -Metadata $media
    }
    
    # Show selection menu
    $selected = Show-FuzzyMenu -Items $results -Header "[SEARCH] Results for: $Query" `
        -DisplayFormatter { 
            param($item)
            $title = if ($item.title.english) { $item.title.english } else { $item.title.romaji }
            $eps = if ($item.episodes) { $item.episodes } else { '?' }
            "$($item.id) - $title ($eps eps) [$($item.status)]"
        } `
        -OnPreview {
            param($item)
            $title = if ($item.title.english) { $item.title.english } else { $item.title.romaji }
            Write-Host "  Title: " -NoNewline -ForegroundColor Green
            Write-Host $title -ForegroundColor White
            
            $eps = if ($item.episodes) { $item.episodes } else { 'Unknown' }
            Write-Host "  Episodes: $eps" -ForegroundColor Gray
            
            Write-Host "  Status: $($item.status)" -ForegroundColor Gray
            
            $score = if ($item.averageScore) { $item.averageScore } else { 'N/A' }
            Write-Host "  Score: $score/100" -ForegroundColor Gray
            if ($item.genres) {
                Write-Host "  Genres: $($item.genres -join ', ')" -ForegroundColor Gray
            }
            if ($item.description) {
                $desc = $item.description.Substring(0, [Math]::Min(200, $item.description.Length))
                Write-Host "  Description: $desc..." -ForegroundColor DarkGray
            }
        }
    
    if (-not $selected) {
        Write-Warn "No anime selected"
        return
    }
    
    $title = if ($selected.title.english) { $selected.title.english } else { $selected.title.romaji }
    Write-Success "Selected: $title"
    
    $episode = Read-Host "Enter episode to watch (or press Enter to skip)"
    
    if ($episode) {
        $coverUrl = $selected.coverImage.large
        Add-WatchHistory -AnimeId $selected.id -Title $title -Episode ([int]$episode) -CoverUrl $coverUrl
        
        # STREAMING LOGIC
        if (Get-Command "ani-cli" -ErrorAction SilentlyContinue) {
            Write-Info "Launching ani-cli for streaming..."
            # Try to pass title and episode. Syntax depends on ani-cli version, but generally "ani-cli title" works.
            # We interactively launch it.
            # Ideally: ani-cli -e <ep> <title>
            Write-Host "Calling: ani-cli -e $episode `"$title`"" -ForegroundColor DarkGray
            
            # Start ani-cli in a new process to ensure it handles TUI correctly
            Start-Process "ani-cli" -ArgumentList "-e", "$episode", "`"$title`"" -Wait -NoNewWindow
        }
        else {
            Write-Success "Recorded: Episode $episode of $title (History Only)"
            Write-Host ""
            Write-Warn "Streamer 'ani-cli' not found."
            Write-Host "To enable video streaming on Windows:" -ForegroundColor Cyan
            Write-Host "  scoop bucket add extras"
            Write-Host "  scoop install ani-cli mpv"
            Write-Host ""
            Write-Host "Press Enter to continue..."
            Read-Host | Out-Null
        }
    }
}

# =============================================================================
# DASHBOARD MODE
# =============================================================================

function Invoke-DashboardMode {
    $history = Get-WatchHistory
    
    if (-not $history -or $history.Count -eq 0) {
        Write-Warn "No watched anime in history."
        Write-Host "Search for anime to add to your watch history."
        return
    }
    
    Write-Info "Loading dashboard with $($history.Count) anime..."
    
    $selected = Show-FuzzyMenu -Items $history -Header "[DASHBOARD] Watched Anime" `
        -DisplayFormatter {
            param($item)
            "$($item.id) - $($item.title) (Ep: $($item.last_episode)) [$($item.last_watched.Substring(0, 10))]"
        } `
        -OnPreview {
            param($item)
            Write-Host "  Title: $($item.title)" -ForegroundColor White
            Write-Host "  Last Episode: $($item.last_episode)" -ForegroundColor Cyan
            Write-Host "  Last Watched: $($item.last_watched)" -ForegroundColor Gray
            
            # Try to get cached metadata for more info
            $cached = Get-CachedMetadata -AnimeId $item.id.ToString()
            if ($cached) {
                Write-Host "  Status: $($cached.status)" -ForegroundColor Gray
                if ($cached.genres) {
                    Write-Host "  Genres: $($cached.genres -join ', ')" -ForegroundColor DarkGray
                }
            }
        }
    
    if (-not $selected) {
        Write-Warn "No anime selected"
        return
    }
    
    Write-Success "Selected: $($selected.title) (Last: Episode $($selected.last_episode))"
    
    $nextEp = $selected.last_episode + 1
    $input = Read-Host "Continue with episode $nextEp? [Y/n/other]"
    
    if ($input -match '^[Nn]$') {
        return
    }
    
    $episode = $nextEp
    if ($input -and $input -notmatch '^[Yy]$') {
        $episode = [int]$input
    }
    
    Add-WatchHistory -AnimeId $selected.id -Title $selected.title -Episode $episode -CoverUrl $selected.cover_url
    Write-Success "Recorded: Episode $episode of $($selected.title)"
}

# =============================================================================
# HELP & VERSION
# =============================================================================

function Show-Help {
    @"
    
ani-tui - Enhanced anime TUI for Windows

USAGE:
    .\ani-tui.ps1                     Interactive search mode
    .\ani-tui.ps1 search "<query>"    Search for anime
    .\ani-tui.ps1 dashboard           View watched anime dashboard

OPTIONS:
    -Help, -h        Show this help message
    -Version, -v     Show version information

EXAMPLES:
    .\ani-tui.ps1                     Start interactive search
    .\ani-tui.ps1 search "naruto"     Search for Naruto
    .\ani-tui.ps1 dashboard           Open watch history dashboard

FILES:
    Cache:   $script:CACHE_DIR
    History: $script:HISTORY_FILE

"@
}

function Show-Version {
    Write-Host "ani-tui version $script:VERSION"
}

# =============================================================================
# MAIN
# =============================================================================

function Main {
    if ($Help) {
        Show-Help
        return
    }
    
    if ($Version) {
        Show-Version
        return
    }
    
    Ensure-Directories
    
    switch ($Command.ToLower()) {
        "search" {
            $query = $Arguments -join " "
            Invoke-SearchMode -Query $query
        }
        "dashboard" {
            Invoke-DashboardMode
        }
        "" {
            Invoke-SearchMode
        }
        default {
            # Treat as search query
            $query = (@($Command) + $Arguments) -join " "
            Invoke-SearchMode -Query $query
        }
    }
}

# Run main
Main
