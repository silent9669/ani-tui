# Providers Module - AGENTS.md

## Overview
Anime source providers. Each provider implements the `AnimeProvider` trait for searching and streaming.

## Structure
```
providers/
├── mod.rs           # Trait definitions + ProviderRegistry
├── allanime.rs      # AllAnime provider (English) - 352 lines
├── kkphim.rs        # KKPhim provider (Vietnamese) - 359 lines
├── gogoanime.rs     # Gogoanime provider - 313 lines
├── hike.rs          # HiAnime provider - 264 lines
└── prowlarr.rs      # Prowlarr integration - 173 lines
```

## Provider Trait
```rust
#[async_trait]
pub trait AnimeProvider: Send + Sync {
    fn name(&self) -> &str;
    fn language(&self) -> Language;
    
    async fn search(&self, query: &str) -> Result<Vec<Anime>>;
    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>>;
    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo>;
}
```

## Provider Registry
- Aggregates all providers
- Filters by language (English/Vietnamese)
- Falls back gracefully on provider errors

## Core Types
```rust
pub struct Anime {
    pub id: String,
    pub provider: String,
    pub title: String,
    pub cover_url: String,
    pub language: Language,
    pub total_episodes: Option<u32>,
    pub synopsis: Option<String>,
}

pub struct Episode {
    pub number: u32,
    pub title: Option<String>,
    pub thumbnail: Option<String>,
}

pub struct StreamInfo {
    pub video_url: String,
    pub subtitles: Vec<Subtitle>,
    pub qualities: Vec<String>,
    pub headers: HashMap<String, String>,
}
```

## Conventions

### Provider Implementation Pattern
```rust
pub struct AllAnimeProvider {
    client: reqwest::Client,
}

const API_BASE: &str = "https://api.example.com";
const REFERRER: &str = "https://example.com";

impl AllAnimeProvider {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("..."));
        headers.insert(REFERER, HeaderValue::from_static(REFERRER));
        
        Self { client: reqwest::Client::builder()... }
    }
}

#[async_trait]
impl AnimeProvider for AllAnimeProvider {
    // Implement trait methods
}
```

### API Client Setup
- Set User-Agent and Referer headers
- Use timeouts (10s default)
- Handle JSON parsing with serde

## Anti-Patterns
- **DON'T** hardcode API URLs in multiple places (use constants)
- **DON'T** panic on API errors (return Result)
- **DON'T** ignore rate limits (respect provider terms)
- **DON'T** break trait contract (all methods must be async)

## Where to Look
| Task | File |
|------|------|
| Add new provider | Create new file + add to mod.rs + register in ProviderRegistry |
| Fix search | Provider's `search()` method |
| Fix streaming | Provider's `get_stream_url()` method |
| Change provider selection | ProviderRegistry in mod.rs |
| Provider-specific decoding | Provider file (e.g., allanime.rs decode functions) |

## Active Providers
- **AllAnime** (English) - Primary English source
- **KKPhim** (Vietnamese) - Primary Vietnamese source

## Dependencies
- `reqwest` - HTTP client
- `async-trait` - Trait async methods
- `serde` - JSON parsing
