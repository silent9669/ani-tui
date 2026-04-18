use super::{Anime, AnimeProvider, Episode, Language, StreamInfo};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;

const CONSUMET_API: &str = "https://api-anime-rouge.vercel.app/aniwatch";

pub struct AniWatchProvider {
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AniSearchResult {
    id: String,
    title: String,
    image: String,
    #[serde(default)]
    type_: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AniEpisode {
    id: String,
    number: u32,
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AniStreamSource {
    url: String,
    #[serde(rename = "isM3U8")]
    is_m3u8: bool,
}

#[derive(Debug, Deserialize)]
struct AniStreamResponse {
    sources: Vec<AniStreamSource>,
}

impl Default for AniWatchProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AniWatchProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }
}

#[async_trait]
impl AnimeProvider for AniWatchProvider {
    fn name(&self) -> &str {
        "AniWatch"
    }

    fn language(&self) -> Language {
        Language::English
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["🇺🇸".to_string()]
    }

    async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        let url = format!("{}/search", CONSUMET_API);

        let response: serde_json::Value = self
            .client
            .get(&url)
            .query(&[("keyword", query), ("page", "1")])
            .send()
            .await
            .context("Failed to search AniWatch")?
            .json()
            .await
            .context("Failed to parse search response")?;

        let mut results = Vec::new();
        if let Some(results_arr) = response["results"].as_array() {
            for result in results_arr {
                let id = result["id"].as_str().unwrap_or_default().to_string();
                let title = result["title"].as_str().unwrap_or_default().to_string();
                let image = result["image"].as_str().unwrap_or_default().to_string();
                
                if !id.is_empty() && !title.is_empty() {
                    results.push(Anime {
                        id,
                        provider: "AniWatch".to_string(),
                        title,
                        cover_url: image,
                        language: Language::English,
                        total_episodes: None,
                        synopsis: None,
                    });
                }
            }
        }

        Ok(results)
    }

    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        let url = format!("{}/info?id={}", CONSUMET_API, anime_id);

        let response: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get anime info")?
            .json()
            .await
            .context("Failed to parse anime info")?;

        let mut episodes = Vec::new();
        if let Some(eps) = response["episodes"].as_array() {
            for ep in eps {
                let number = ep["number"].as_u64().unwrap_or(0) as u32;
                let id = ep["id"].as_str().unwrap_or_default().to_string();
                let title = ep["title"].as_str().map(|s| s.to_string());
                
                if !id.is_empty() {
                    episodes.push(Episode {
                        number,
                        title,
                        thumbnail: None,
                    });
                }
            }
        }

        Ok(episodes)
    }

    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo> {
        // episode_id for AniWatch in Consumet is usually like "anime-id?ep=123"
        let url = format!("{}/watch?episodeId={}", CONSUMET_API, episode_id);
        
        let stream_resp: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get stream URL")?
            .json()
            .await
            .context("Failed to parse stream response")?;

        let source = stream_resp["sources"]
            .as_array()
            .and_then(|s| s.first())
            .ok_or_else(|| anyhow::anyhow!("No stream sources found"))?;

        let video_url = source["url"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Source URL not found"))?
            .to_string();

        let mut headers = HashMap::new();
        headers.insert("Referer".to_string(), "https://aniwatch.to/".to_string());

        Ok(StreamInfo {
            video_url,
            subtitles: vec![],
            qualities: vec!["auto".to_string()],
            headers,
        })
    }
}
