use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const ANILIST_API: &str = "https://graphql.anilist.co";
const CACHE_TTL_DAYS: i64 = 7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AniListMetadata {
    pub anilist_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub rating: Option<i64>,
    pub cover_url: Option<String>,
    pub genres: Vec<String>,
    pub episode_count: Option<i64>,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct EnrichedAnime {
    pub base: crate::providers::Anime,
    pub metadata: Option<AniListMetadata>,
}

#[derive(Clone)]
pub struct AniListClient {
    client: reqwest::Client,
}

impl AniListClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    pub async fn search_anime(&self, query: &str) -> Result<Vec<AniListMetadata>> {
        let search_query = r#"
            query ($search: String) {
                Page(page: 1, perPage: 10) {
                    media(search: $search, type: ANIME) {
                        id
                        title {
                            romaji
                            english
                            native
                        }
                        description
                        averageScore
                        coverImage {
                            large
                            medium
                        }
                        genres
                        episodes
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "search": query
        });

        let response = self
            .client
            .post(ANILIST_API)
            .json(&serde_json::json!({
                "query": search_query,
                "variables": variables
            }))
            .send()
            .await
            .context("Failed to query AniList")?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("AniList API error: {} - {}", status, text);
        }

        let json: serde_json::Value = response.json().await?;
        let mut results = Vec::new();

        if let Some(media_list) = json["data"]["Page"]["media"].as_array() {
            for media in media_list {
                let anilist_id = media["id"].as_i64().unwrap_or_default();

                let title = media["title"]["english"]
                    .as_str()
                    .or_else(|| media["title"]["romaji"].as_str())
                    .unwrap_or("Unknown")
                    .to_string();

                let description = media["description"]
                    .as_str()
                    .map(|s| s.replace("<br>", "\n").replace("<br/>", "\n"));

                let rating = media["averageScore"].as_i64();

                let cover_url = media["coverImage"]["large"]
                    .as_str()
                    .or_else(|| media["coverImage"]["medium"].as_str())
                    .map(|s| s.to_string());

                let genres: Vec<String> = media["genres"]
                    .as_array()
                    .map(|g| {
                        g.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let episode_count = media["episodes"].as_i64();

                results.push(AniListMetadata {
                    anilist_id,
                    title,
                    description,
                    rating,
                    cover_url,
                    genres,
                    episode_count,
                    cached_at: Utc::now(),
                });
            }
        }

        Ok(results)
    }

    pub async fn get_by_id(&self, anilist_id: i64) -> Result<Option<AniListMetadata>> {
        let query = r#"
            query ($id: Int) {
                Media(id: $id, type: ANIME) {
                    id
                    title {
                        romaji
                        english
                        native
                    }
                    description
                    averageScore
                    coverImage {
                        large
                        medium
                    }
                    genres
                    episodes
                }
            }
        "#;

        let variables = serde_json::json!({
            "id": anilist_id
        });

        let response = self
            .client
            .post(ANILIST_API)
            .json(&serde_json::json!({
                "query": query,
                "variables": variables
            }))
            .send()
            .await
            .context("Failed to query AniList")?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("AniList API error: {} - {}", status, text);
        }

        let json: serde_json::Value = response.json().await?;

        if let Some(media) = json["data"]["Media"].as_object() {
            let anilist_id = media["id"].as_i64().unwrap_or_default();

            let title = media["title"]["english"]
                .as_str()
                .or_else(|| media["title"]["romaji"].as_str())
                .unwrap_or("Unknown")
                .to_string();

            let description = media["description"]
                .as_str()
                .map(|s| s.replace("<br>", "\n").replace("<br/>", "\n"));

            let rating = media["averageScore"].as_i64();

            let cover_url = media["coverImage"]["large"]
                .as_str()
                .or_else(|| media["coverImage"]["medium"].as_str())
                .map(|s| s.to_string());

            let genres: Vec<String> = media["genres"]
                .as_array()
                .map(|g| {
                    g.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let episode_count = media["episodes"].as_i64();

            Ok(Some(AniListMetadata {
                anilist_id,
                title,
                description,
                rating,
                cover_url,
                genres,
                episode_count,
                cached_at: Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone)]
pub struct MetadataCache {
    db: Arc<crate::db::Database>,
    client: AniListClient,
}

impl MetadataCache {
    pub fn new(db: Arc<crate::db::Database>) -> Self {
        Self {
            db,
            client: AniListClient::new(),
        }
    }

    pub async fn get_metadata(&self, anilist_id: i64) -> Result<Option<AniListMetadata>> {
        // Try cache first
        if let Some(cached) = self.db.get_cached_metadata(anilist_id).await? {
            // Check if cache is still valid (7 days)
            if Utc::now()
                .signed_duration_since(cached.cached_at)
                .num_days()
                < CACHE_TTL_DAYS
            {
                return Ok(Some(cached));
            }
        }

        // Fetch from API
        match self.client.get_by_id(anilist_id).await {
            Ok(Some(metadata)) => {
                // Cache the result
                let _ = self.db.cache_metadata(&metadata).await;
                Ok(Some(metadata))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                tracing::warn!("Failed to fetch metadata from AniList: {}", e);
                // Return cached data even if expired as fallback
                Ok(self.db.get_cached_metadata(anilist_id).await?)
            }
        }
    }

    pub async fn search_and_cache(&self, query: &str) -> Result<Vec<AniListMetadata>> {
        tracing::info!("Searching AniList for: {}", query);

        let results = self.client.search_anime(query).await?;

        tracing::info!("AniList returned {} results for '{}'", results.len(), query);

        // Cache all results
        for metadata in &results {
            tracing::debug!(
                "Caching metadata for: {} (AniList ID: {})",
                metadata.title,
                metadata.anilist_id
            );
            let _ = self.db.cache_metadata(metadata).await;
        }

        Ok(results)
    }

    pub async fn enrich_anime(&self, base: crate::providers::Anime) -> EnrichedAnime {
        // Search for matching metadata
        match self.search_and_cache(&base.title).await {
            Ok(results) => {
                // Find best match (first result is usually best)
                let metadata = results.into_iter().next();
                EnrichedAnime { base, metadata }
            }
            Err(e) => {
                tracing::warn!("Failed to enrich anime '{}': {}", base.title, e);
                EnrichedAnime {
                    base,
                    metadata: None,
                }
            }
        }
    }

    pub async fn enrich_anime_list(
        &self,
        anime_list: Vec<crate::providers::Anime>,
    ) -> Vec<EnrichedAnime> {
        let mut enriched = Vec::new();

        for anime in anime_list {
            enriched.push(self.enrich_anime(anime).await);
        }

        enriched
    }
}

impl Default for AniListClient {
    fn default() -> Self {
        Self::new()
    }
}
