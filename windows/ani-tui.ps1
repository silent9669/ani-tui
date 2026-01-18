<#
.SYNOPSIS
    ani-tui v5.4 - Enhanced Anime TUI for Windows (macOS parity)
#>

param(
    [Parameter(Position = 0)][string]$Command = "",
    [Parameter(Position = 1, ValueFromRemainingArguments)][string[]]$Arguments
)

$VERSION = "5.4.0"
$DATA_DIR = "$env:USERPROFILE\.ani-tui"
$CACHE_DIR = "$DATA_DIR\cache"
$IMAGES_DIR = "$CACHE_DIR\images"
$HISTORY_FILE = "$DATA_DIR\history.json"

$ALLANIME_API = "https://api.allanime.day"
$ALLANIME_REFR = "https://allmanga.to"
$ANILIST_API = "https://graphql.anilist.co"

# Catppuccin colors
$FZF_COLORS = "fg:#cdd6f4,bg:#1e1e2e,hl:#f9e2af,fg+:#cdd6f4,bg+:#313244,hl+:#f9e2af,info:#94e2d5,prompt:#f5c2e7,pointer:#f5e0dc,marker:#a6e3a1,spinner:#f5e0dc,header:#89b4fa,border:#6c7086"

# =============================================================================
# INIT
# =============================================================================
function Init {
    @($DATA_DIR, $CACHE_DIR, $IMAGES_DIR) | ForEach-Object {
        if (-not (Test-Path $_)) { New-Item -ItemType Directory -Path $_ -Force | Out-Null }
    }
    if (-not (Test-Path $HISTORY_FILE)) { "[]" | Out-File $HISTORY_FILE -Encoding UTF8 }
}

# =============================================================================
# API
# =============================================================================
function SearchAnime($q) {
    if (!$q -or $q.Length -lt 2) { return }
    $gql = 'query($search:SearchInput$limit:Int$page:Int$translationType:VaildTranslationTypeEnumType$countryOrigin:VaildCountryOriginEnumType){shows(search:$search limit:$limit page:$page translationType:$translationType countryOrigin:$countryOrigin){edges{_id name availableEpisodes __typename}}}'
    $vars = (@{search=@{allowAdult=$false;allowUnknown=$false;query=$q};limit=30;page=1;translationType="sub";countryOrigin="ALL"} | ConvertTo-Json -Compress)
    try {
        $r = Invoke-RestMethod "$ALLANIME_API/api?variables=$([uri]::EscapeDataString($vars))&query=$([uri]::EscapeDataString($gql))" -Headers @{"User-Agent"="Mozilla/5.0";"Referer"=$ALLANIME_REFR} -TimeoutSec 10
        foreach ($s in $r.data.shows.edges) {
            if ($s.availableEpisodes.sub -gt 0) { "$($s._id)`t$($s.name) ($($s.availableEpisodes.sub) eps)" }
        }
    } catch {}
}

function GetEpisodes($id) {
    $gql = 'query($showId:String!){show(_id:$showId){_id availableEpisodesDetail}}'
    $vars = (@{showId=$id} | ConvertTo-Json -Compress)
    try {
        $r = Invoke-RestMethod "$ALLANIME_API/api?variables=$([uri]::EscapeDataString($vars))&query=$([uri]::EscapeDataString($gql))" -Headers @{"User-Agent"="Mozilla/5.0";"Referer"=$ALLANIME_REFR} -TimeoutSec 10
        $r.data.show.availableEpisodesDetail.sub | Sort-Object {[double]$_}
    } catch {}
}

# =============================================================================
# HISTORY
# =============================================================================
function GetHistory {
    try {
        $c = Get-Content $HISTORY_FILE -Raw
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
        if ($h[$i].title -eq $title) { $h[$i].last_episode=$ep; $h[$i].last_watched=$ts; $found=$true; break }
    }
    if (!$found) { $h += [PSCustomObject]@{title=$title;last_episode=$ep;last_watched=$ts} }
    $h | ConvertTo-Json -Depth 10 | Out-File $HISTORY_FILE -Encoding UTF8
}

function DeleteHistory($title) {
    $h = @(GetHistory) | Where-Object { $_.title -ne $title }
    @($h) | ConvertTo-Json -Depth 10 | Out-File $HISTORY_FILE -Encoding UTF8
}

# =============================================================================
# PREVIEW - fetch cover and display with chafa
# =============================================================================
function ShowPreview($input) {
    if (!$input) { return }
    
    # Clean title
    $name = $input -replace '^HIST\s*','' -replace '^\[\d+\]\s*','' -replace '\s*\(\d+\s+eps\)$','' -replace '\s*$',''
    if (!$name) { return }
    
    Write-Host ""
    Write-Host "  $name" -ForegroundColor White
    Write-Host ""
    
    # Try to show cached image
    $hash = [BitConverter]::ToString([Security.Cryptography.MD5]::Create().ComputeHash([Text.Encoding]::UTF8.GetBytes($name))).Replace("-","").Substring(0,12).ToLower()
    $img = "$IMAGES_DIR\$hash.jpg"
    
    if (-not (Test-Path $img)) {
        # Fetch from AniList
        try {
            $body = @{query="query{Page(perPage:1){media(search:`"$($name -replace '"','\"')`",type:ANIME){coverImage{large}}}}"} | ConvertTo-Json -Compress
            $r = Invoke-RestMethod $ANILIST_API -Method Post -ContentType "application/json" -Body $body -TimeoutSec 5
            $cover = $r.data.Page.media[0].coverImage.large
            if ($cover) { Invoke-WebRequest $cover -OutFile $img -TimeoutSec 8 | Out-Null }
        } catch {}
    }
    
    if ((Test-Path $img) -and (Get-Command "chafa" -ErrorAction SilentlyContinue)) {
        & chafa --size=50x30 $img 2>$null
    } else {
        Write-Host "  (Install chafa for image preview)" -ForegroundColor DarkGray
    }
}

# =============================================================================
# COMMAND HANDLERS (for fzf reload/preview)
# =============================================================================

# --search: real-time search (called by fzf reload)
if ($Command -eq "--search") {
    $q = $Arguments -join " "
    if ($q -and $q.Length -ge 2) {
        SearchAnime $q
    } else {
        # Empty query = show history
        Init
        GetHistory | Select-Object -First 10 | ForEach-Object {
            "HIST`t[$($_.last_episode)] $($_.title)"
        }
    }
    exit
}

# --history: show history
if ($Command -eq "--history") {
    Init
    GetHistory | Select-Object -First 10 | ForEach-Object {
        "HIST`t[$($_.last_episode)] $($_.title)"
    }
    exit
}

# --preview: show image preview
if ($Command -eq "--preview") {
    Init
    ShowPreview ($Arguments -join " ")
    exit
}

# --delete: remove from history
if ($Command -eq "--delete") {
    Init
    $t = ($Arguments -join " ") -replace '^\[\d+\]\s*',''
    DeleteHistory $t
    exit
}

# =============================================================================
# MAIN TUI
# =============================================================================
function StartTUI {
    Init
    
    if (-not (Get-Command "fzf" -ErrorAction SilentlyContinue)) {
        Write-Host "fzf not found. Install: scoop install fzf" -ForegroundColor Red
        exit 1
    }
    
    # Get script path for reload commands
    $script = $PSCommandPath
    if (!$script) { $script = $MyInvocation.MyCommand.Path }
    
    while ($true) {
        # Initial items = history
        $items = @()
        GetHistory | Select-Object -First 10 | ForEach-Object {
            $items += "HIST`t[$($_.last_episode)] $($_.title)"
        }
        
        # fzf with real-time reload
        $header = "ani-tui | Type to search | Enter Select | Ctrl-D Delete | Esc Quit"
        
        # Build reload command - use powershell directly with proper escaping
        $reloadCmd = "powershell -NoProfile -NoLogo -ExecutionPolicy Bypass -File \`"$script\`" --search {q}"
        $historyCmd = "powershell -NoProfile -NoLogo -ExecutionPolicy Bypass -File \`"$script\`" --history"
        $deleteCmd = "powershell -NoProfile -NoLogo -ExecutionPolicy Bypass -File \`"$script\`" --delete {2}"
        $previewCmd = "powershell -NoProfile -NoLogo -ExecutionPolicy Bypass -File \`"$script\`" --preview {2}"
        
        $selected = ($items -join "`n") | fzf --ansi --height=100% --layout=reverse --border=rounded --padding=1 `
            --delimiter="`t" --with-nth=2 `
            --header="$header" --header-first `
            --prompt="Search: " --pointer=">" `
            --preview="$previewCmd" --preview-window="right:50%:wrap" `
            --bind="change:reload:$reloadCmd" `
            --bind="ctrl-d:execute-silent($deleteCmd)+reload($historyCmd)" `
            --color="$FZF_COLORS" 2>$null
        
        if (!$selected) { break }
        
        # Parse selection
        $parts = $selected -split "`t", 2
        $showId = $null
        $title = $null
        
        if ($selected -match "^HIST") {
            $title = ($parts[1] -replace '^\[\d+\]\s*','').Trim()
            Write-Host "`nSearching: $title" -ForegroundColor Cyan
            $results = @(SearchAnime $title)
            if ($results.Count -gt 0) { $showId = ($results[0] -split "`t")[0] }
        } else {
            $showId = $parts[0]
            $title = ($parts[1] -replace '\s*\(\d+\s+eps\)$','').Trim()
        }
        
        if (!$showId) {
            Write-Host "Could not find anime" -ForegroundColor Yellow
            Start-Sleep 1
            continue
        }
        
        # Episodes
        Write-Host "Loading episodes..." -ForegroundColor Cyan
        $episodes = @(GetEpisodes $showId)
        
        if ($episodes.Count -eq 0) {
            Write-Host "No episodes" -ForegroundColor Yellow
            Start-Sleep 1
            continue
        }
        
        # Last watched
        $lastEp = 0
        GetHistory | ForEach-Object { if ($_.title -eq $title) { $lastEp = [int]$_.last_episode } }
        
        # Format episodes
        $epItems = foreach ($ep in $episodes) {
            $n = [int]$ep
            if ($n -eq $lastEp) { "$ep  << Last" }
            elseif ($n -eq $lastEp + 1) { "$ep  >> Next" }
            else { "$ep" }
        }
        
        $epHeader = "$title | Enter=Play | Esc=Back"
        $epChoice = $epItems | fzf --ansi --height=100% --layout=reverse --border=rounded --padding=1 `
            --header="$epHeader" --header-first --prompt="Episode: " --pointer=">" --no-info `
            --color="fg:#cdd6f4,bg:#1e1e2e,hl:#a6e3a1,fg+:#cdd6f4,bg+:#313244,hl+:#a6e3a1,prompt:#94e2d5,pointer:#f5e0dc,header:#cba6f7,border:#6c7086"
        
        if (!$epChoice) { continue }
        
        $episode = [int](($epChoice -split '\s+')[0])
        SaveHistory $title $episode
        
        Clear-Host
        Write-Host "`n  > Playing: $title - Episode $episode`n" -ForegroundColor Green
        
        if (Get-Command "ani-cli" -ErrorAction SilentlyContinue) {
            & ani-cli -S 1 -e $episode "$title"
        } else {
            Write-Host "  ani-cli not found. Install: scoop install ani-cli mpv" -ForegroundColor Yellow
            Write-Host "  Episode saved to history.`n" -ForegroundColor Cyan
            Read-Host "  Press Enter"
        }
    }
}

# =============================================================================
# ENTRY POINT
# =============================================================================
switch ($Command.ToLower()) {
    "-h" { Write-Host "`nani-tui v$VERSION - scoop install fzf chafa ani-cli mpv`n" }
    "--help" { Write-Host "`nani-tui v$VERSION - scoop install fzf chafa ani-cli mpv`n" }
    "-v" { Write-Host "ani-tui $VERSION" }
    "--version" { Write-Host "ani-tui $VERSION" }
    default { StartTUI }
}
