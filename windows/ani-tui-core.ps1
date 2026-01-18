<#
.SYNOPSIS
    ani-tui v6.2 for Windows - Fixed Space Handling & Preview
.DESCRIPTION
    Uses native batch/curl for fzf callbacks. Fixed issues with:
    - Space characters in search queries
    - Empty search showing history
    - Image preview functionality
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
$script:VERSION = "6.3.0"
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
    
    Create-HelperScripts
}

function Create-HelperScripts {
    # ==========================================================================
    # SEARCH HELPER - Fixed space handling with proper quoting
    # ==========================================================================
    # Using PowerShell for search since batch has issues with special chars
    $searchScript = @'
[CmdletBinding()]
param([Parameter(ValueFromRemainingArguments)][string[]]$Args)
$ErrorActionPreference = "SilentlyContinue"
$q = ($Args -join " ").Trim()
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
    
    # Batch wrapper that calls PowerShell with proper quoting
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
param([Parameter(ValueFromRemainingArguments)][string[]]$Args)
$ErrorActionPreference = "SilentlyContinue"
$input = ($Args -join " ").Trim()
$HIST = "$env:USERPROFILE\.ani-tui\history.json"

# Extract title (remove [xx] prefix)
$title = $input -replace '^\[\d+\]\s*', ''
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
    # PREVIEW HELPER - Fixed to properly fetch and display images
    # ==========================================================================
    $previewScript = @'
[CmdletBinding()]
param([Parameter(ValueFromRemainingArguments)][string[]]$Args)
$ErrorActionPreference = "SilentlyContinue"
$input = ($Args -join " ").Trim()
if (!$input) { exit }

$IMAGES = "$env:USERPROFILE\.ani-tui\cache\images"
if (!(Test-Path $IMAGES)) { New-Item -ItemType Directory -Path $IMAGES -Force | Out-Null }

# Clean title
$name = $input
$name = $name -replace '^HIST\s*', ''
$name = $name -replace '^\[\d+\]\s*', ''
$name = $name -replace '\s*\(\d+\s+eps\)\s*$', ''
$name = $name -replace '\s*\[[A-Z]+\]\s*$', ''
$name = $name.Trim()

if (!$name) { exit }

# Display title
Write-Host ""
Write-Host "  $name" -ForegroundColor Cyan
Write-Host ""

# Check for chafa
$hasChafa = Get-Command chafa -ErrorAction SilentlyContinue
if (!$hasChafa) {
    Write-Host "  [Install chafa for image preview]" -ForegroundColor DarkGray
    Write-Host "  scoop install chafa" -ForegroundColor DarkGray
    exit
}

# Generate hash for cache
$bytes = [System.Text.Encoding]::UTF8.GetBytes($name)
$hash = [System.BitConverter]::ToString(
    [System.Security.Cryptography.MD5]::Create().ComputeHash($bytes)
).Replace("-", "").Substring(0, 12).ToLower()
$imgPath = "$IMAGES\$hash.jpg"

# Fetch image if not cached (use extraLarge for better quality)
if (!(Test-Path $imgPath)) {
    try {
        $query = @{ query = "query{Page(perPage:1){media(search:`"$name`",type:ANIME){coverImage{extraLarge large}}}}" } | ConvertTo-Json
        $r = Invoke-RestMethod "https://graphql.anilist.co" -Method Post -ContentType "application/json" -Body $query -TimeoutSec 8
        $coverUrl = $r.data.Page.media[0].coverImage.extraLarge
        if (!$coverUrl) { $coverUrl = $r.data.Page.media[0].coverImage.large }
        if ($coverUrl) {
            Invoke-WebRequest $coverUrl -OutFile $imgPath -TimeoutSec 10
        }
    } catch {
        Write-Host "  Loading..." -ForegroundColor DarkGray
        exit
    }
}

# Display image with high quality settings
if (Test-Path $imgPath) {
    & chafa --size=60x35 --symbols=all --colors=256 $imgPath 2>$null
}
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
        
        $aniCli = Get-Command ani-cli -ErrorAction SilentlyContinue
        if ($aniCli) {
            Write-Host "  Running: ani-cli -S 1 -e $episode `"$title`"" -ForegroundColor DarkGray
            Write-Host ""
            
            # Use cmd.exe /c to run ani-cli to avoid WSL issues
            # This ensures it runs in native Windows mode
            $cmdArgs = "ani-cli -S 1 -e $episode `"$title`""
            Start-Process -FilePath "cmd.exe" -ArgumentList "/c", $cmdArgs -Wait -NoNewWindow
            
            Write-Host ""
            Write-Host "  Playback ended." -ForegroundColor Cyan
            Read-Host "  Press Enter to continue"
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
