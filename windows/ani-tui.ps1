<#
.SYNOPSIS
    ani-tui v5.1 - Enhanced Anime TUI for Windows
    
.DESCRIPTION
    Terminal-based anime browser with fzf interface, image previews, and streaming.
    Windows PowerShell implementation matching macOS functionality.
    
.EXAMPLE
    ani-tui
    Interactive TUI mode with watch history
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

$script:VERSION = "5.1.0"

# Directories (matching macOS paths style)
$script:DATA_DIR = Join-Path $env:USERPROFILE ".ani-tui"
$script:CACHE_DIR = Join-Path $script:DATA_DIR "cache"
$script:IMAGES_DIR = Join-Path $script:CACHE_DIR "images"
$script:HISTORY_FILE = Join-Path $script:DATA_DIR "history.json"

# AllAnime API (same as macOS)
$script:ALLANIME_API = "https://api.allanime.day"
$script:ALLANIME_REFR = "https://allmanga.to"
$script:ANILIST_API = "https://graphql.anilist.co"
$script:USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/121.0"

# Catppuccin Mocha colors for fzf (same as macOS)
$script:FZF_COLORS = "fg:#cdd6f4,bg:#1e1e2e,hl:#f9e2af,fg+:#cdd6f4,bg+:#313244,hl+:#f9e2af,info:#94e2d5,prompt:#f5c2e7,pointer:#f5e0dc,marker:#a6e3a1,spinner:#f5e0dc,header:#89b4fa,border:#6c7086"

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
    $missing = @()
    
    if (-not (Get-Command "fzf" -ErrorAction SilentlyContinue)) {
        $missing += "fzf"
    }
    if (-not (Get-Command "curl" -ErrorAction SilentlyContinue)) {
        $missing += "curl"
    }
    
    if ($missing.Count -gt 0) {
        Write-Host "Missing dependencies: $($missing -join ', ')" -ForegroundColor Red
        Write-Host "Install via: scoop install $($missing -join ' ')" -ForegroundColor Yellow
        exit 1
    }
}

# =============================================================================
# ALLANIME API FUNCTIONS (matching macOS)
# =============================================================================

function Search-AllAnime {
    param([string]$Query)
    
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
        } -TimeoutSec 10
        
        $results = @()
        if ($response.data.shows.edges) {
            foreach ($show in $response.data.shows.edges) {
                $eps = $show.availableEpisodes.sub
                if ($eps -and $eps -gt 0) {
                    $results += "$($show._id)`t$($show.name) ($eps episodes)"
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
        } -TimeoutSec 10
        
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
    param(
        [string]$Title,
        [int]$Episode
    )
    
    $timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $history = Get-WatchHistory
    
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
        $history += @{
            title = $Title
            last_episode = $Episode
            last_watched = $timestamp
        }
    }
    
    $history | ConvertTo-Json -Depth 10 | Out-File -FilePath $script:HISTORY_FILE -Encoding UTF8
}

function Remove-FromHistory {
    param([string]$Title)
    
    $history = Get-WatchHistory
    $history = $history | Where-Object { $_.title -ne $Title }
    if ($null -eq $history) { $history = @() }
    $history | ConvertTo-Json -Depth 10 | Out-File -FilePath $script:HISTORY_FILE -Encoding UTF8
}

# =============================================================================
# PREVIEW SCRIPT CREATION
# =============================================================================

function Create-PreviewScript {
    $previewScript = @'
param([string]$Input)

if ([string]::IsNullOrWhiteSpace($Input)) { exit 0 }

$CACHE = Join-Path $env:USERPROFILE ".ani-tui\cache\images"
if (-not (Test-Path $CACHE)) { New-Item -ItemType Directory -Path $CACHE -Force | Out-Null }

# Extract title - remove HIST prefix and episode markers
$name = $Input
$name = $name -replace '^HIST\s*', ''
$name = $name -replace '^\[\d+\]\s*', ''
$name = $name -replace '\s+\d+\s+eps($|\s)', ' '
$name = $name -replace '\s*\(\d+\s+episodes\)$', ''
$name = $name -replace '\s*\[[A-Z]+\]\s*$', ''
$name = $name -replace '\s*-\s*$', ''
$name = $name.Trim()

if ([string]::IsNullOrWhiteSpace($name)) { exit 0 }

# Fetch cover from AniList
$gql = @"
query {
    Page(perPage: 1) {
        media(search: "$($name -replace '"', '\"')", type: ANIME) {
            coverImage { extraLarge large }
        }
    }
}
"@

try {
    $body = @{ query = $gql } | ConvertTo-Json -Compress
    $response = Invoke-RestMethod -Uri "https://graphql.anilist.co" -Method Post -ContentType "application/json" -Body $body -TimeoutSec 5
    
    $cover = $response.data.Page.media[0].coverImage.extraLarge
    if (-not $cover) { $cover = $response.data.Page.media[0].coverImage.large }
    
    if ($cover) {
        # Create hash for filename
        $hash = [System.BitConverter]::ToString([System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($name))).Replace("-", "").Substring(0, 12).ToLower()
        $imgPath = Join-Path $CACHE "$hash.jpg"
        
        if (-not (Test-Path $imgPath)) {
            Invoke-WebRequest -Uri $cover -OutFile $imgPath -TimeoutSec 10 | Out-Null
        }
        
        # Render with chafa if available
        if (Get-Command "chafa" -ErrorAction SilentlyContinue) {
            Write-Host ""
            Write-Host ""
            & chafa --size=60x35 --symbols=all --colors=256 $imgPath 2>$null
        }
        else {
            Write-Host ""
            Write-Host "  $name" -ForegroundColor White
            Write-Host ""
            Write-Host "  (Install chafa for image previews: scoop install chafa)" -ForegroundColor DarkGray
        }
    }
    else {
        Write-Host ""
        Write-Host "  $name" -ForegroundColor White
        Write-Host ""
        Write-Host "  Loading..." -ForegroundColor DarkGray
    }
}
catch {
    Write-Host ""
    Write-Host "  $name" -ForegroundColor White
}
'@

    $scriptPath = Join-Path $script:CACHE_DIR "preview.ps1"
    $previewScript | Out-File -FilePath $scriptPath -Encoding UTF8
    return $scriptPath
}

# =============================================================================
# SEARCH HELPER (for fzf reload)
# =============================================================================

function Invoke-SearchHelper {
    param([string]$Query)
    
    if ([string]::IsNullOrWhiteSpace($Query) -or $Query.Length -lt 2) {
        exit 0
    }
    
    $results = Search-AllAnime -Query $Query
    $results | ForEach-Object { Write-Output $_ }
    exit 0
}

# =============================================================================
# MAIN TUI
# =============================================================================

function Start-TUI {
    Initialize-Directories
    Test-Dependencies
    
    $previewScript = Create-PreviewScript
    $scriptPath = $MyInvocation.MyCommand.Path
    if (-not $scriptPath) { $scriptPath = $PSCommandPath }
    
    while ($true) {
        # Build initial list from history
        $history = Get-WatchHistory
        $items = @()
        $history | Select-Object -First 10 | ForEach-Object {
            $items += "HIST`t[$($_.last_episode)] $($_.title)"
        }
        
        # Create temp file for initial items
        $tempFile = [System.IO.Path]::GetTempFileName()
        if ($items.Count -gt 0) {
            $items -join "`n" | Out-File -FilePath $tempFile -Encoding UTF8 -NoNewline
        }
        else {
            "" | Out-File -FilePath $tempFile -Encoding UTF8 -NoNewline
        }
        
        # Build fzf command with real-time search
        $fzfHeader = "ani-tui | Type to search | Up/Down Navigate | Enter Select | Ctrl-D Delete | Esc Quit"
        
        # Use cmd to handle the reload properly on Windows
        $reloadCmd = "powershell -NoProfile -ExecutionPolicy Bypass -File `"$scriptPath`" --search {q}"
        $deleteCmd = "powershell -NoProfile -ExecutionPolicy Bypass -File `"$scriptPath`" --delete {2}"
        $historyReload = "powershell -NoProfile -ExecutionPolicy Bypass -File `"$scriptPath`" --history"
        
        $selected = Get-Content $tempFile -Raw | fzf `
            --ansi `
            --height=100% `
            --layout=reverse `
            --border=rounded `
            --margin=0 `
            --padding=1 `
            --delimiter="`t" `
            --with-nth=2 `
            --header="$fzfHeader" `
            --header-first `
            --prompt="Search: " `
            --pointer=">" `
            --preview="powershell -NoProfile -ExecutionPolicy Bypass -File `"$previewScript`" {2}" `
            --preview-window="right:50%:border-rounded" `
            --bind="change:reload:$reloadCmd" `
            --bind="ctrl-d:execute-silent($deleteCmd)+reload($historyReload)" `
            --color="$script:FZF_COLORS"
        
        Remove-Item $tempFile -Force -ErrorAction SilentlyContinue
        
        if ([string]::IsNullOrWhiteSpace($selected)) { break }
        
        # Parse selection
        $parts = $selected -split "`t", 2
        $showId = $null
        $title = $null
        
        if ($selected -match "^HIST") {
            # From history - need to search for the show
            $title = $parts[1] -replace '^\[\d+\]\s*', ''
            Write-Host "Searching: $title" -ForegroundColor Blue
            $searchResult = Search-AllAnime -Query $title
            if ($searchResult.Count -gt 0) {
                $showId = ($searchResult[0] -split "`t")[0]
            }
        }
        else {
            # From search
            $showId = $parts[0]
            $title = $parts[1] -replace '\s*\(\d+\s+episodes\)$', ''
        }
        
        if ([string]::IsNullOrWhiteSpace($showId)) { continue }
        
        # Get episodes
        Write-Host "Loading episodes..." -ForegroundColor Blue
        $episodes = Get-Episodes -ShowId $showId
        
        if ($episodes.Count -eq 0) {
            Write-Host "No episodes available" -ForegroundColor Yellow
            Start-Sleep -Seconds 1
            continue
        }
        
        # Get last watched episode
        $lastEp = 0
        $history | ForEach-Object {
            if ($_.title -eq $title) {
                $lastEp = [int]$_.last_episode
            }
        }
        
        # Format episodes with markers
        $epList = @()
        foreach ($ep in $episodes) {
            $epNum = [int]$ep
            if ($epNum -eq $lastEp) {
                $epList += "$ep  < Last watched"
            }
            elseif ($epNum -eq ($lastEp + 1)) {
                $epList += "$ep  * Continue"
            }
            else {
                $epList += "$ep"
            }
        }
        
        # Episode selection
        $epChoice = $epList | fzf `
            --ansi `
            --height=100% `
            --layout=reverse `
            --border=rounded `
            --padding=1 `
            --header="$title`n`nEnter to PLAY | Esc BACK" `
            --header-first `
            --prompt="Episode: " `
            --pointer=">" `
            --preview="powershell -NoProfile -ExecutionPolicy Bypass -File `"$previewScript`" `"$title`"" `
            --preview-window="right:55%:border-rounded" `
            --color="fg:#cdd6f4,bg:#1e1e2e,hl:#a6e3a1,fg+:#cdd6f4,bg+:#313244,hl+:#a6e3a1,prompt:#94e2d5,pointer:#f5e0dc,header:#cba6f7,border:#6c7086"
        
        if ([string]::IsNullOrWhiteSpace($epChoice)) { continue }
        
        # Extract episode number
        $episode = ($epChoice -split '\s+')[0]
        
        # Update history
        Update-WatchHistory -Title $title -Episode ([int]$episode)
        
        # Play with ani-cli
        Clear-Host
        Write-Host "> Now Playing: $title - Episode $episode" -ForegroundColor Green
        Write-Host ""
        
        if (Get-Command "ani-cli" -ErrorAction SilentlyContinue) {
            & ani-cli -S 1 -e $episode "$title"
        }
        else {
            Write-Host "ani-cli not found. Install with: scoop install ani-cli" -ForegroundColor Yellow
            Write-Host "Episode $episode of $title recorded in history." -ForegroundColor Cyan
            Write-Host ""
            Write-Host "Press Enter to continue..."
            Read-Host | Out-Null
        }
    }
}

# =============================================================================
# COMMAND HANDLERS (for fzf callbacks)
# =============================================================================

if ($Command -eq "--search") {
    $query = $Arguments -join " "
    Invoke-SearchHelper -Query $query
    exit 0
}

if ($Command -eq "--delete") {
    $titleArg = $Arguments -join " "
    $titleClean = $titleArg -replace '^\[\d+\]\s*', ''
    Remove-FromHistory -Title $titleClean
    exit 0
}

if ($Command -eq "--history") {
    Initialize-Directories
    $history = Get-WatchHistory
    $history | Select-Object -First 10 | ForEach-Object {
        Write-Output "HIST`t[$($_.last_episode)] $($_.title)"
    }
    exit 0
}

# =============================================================================
# HELP & VERSION
# =============================================================================

function Show-Help {
    Write-Host @"

ani-tui v$script:VERSION - Anime TUI for Windows

USAGE:
    ani-tui                     Interactive TUI mode

CONTROLS:
    Type        Search anime (real-time)
    Up/Down     Navigate list
    Enter       Select/Play
    Ctrl-D      Delete from history
    Esc         Back/Quit

FILES:
    Cache:   $script:CACHE_DIR
    History: $script:HISTORY_FILE

DEPENDENCIES:
    Required: fzf, curl
    Optional: chafa (image previews), ani-cli + mpv (streaming)

INSTALL DEPS:
    scoop install fzf chafa ani-cli mpv

"@
}

function Show-Version {
    Write-Host "ani-tui $script:VERSION"
}

# =============================================================================
# MAIN
# =============================================================================

if ($Help) {
    Show-Help
    exit 0
}

if ($Version) {
    Show-Version
    exit 0
}

switch ($Command.ToLower()) {
    "-h" { Show-Help }
    "--help" { Show-Help }
    "-v" { Show-Version }
    "--version" { Show-Version }
    default { Start-TUI }
}
