use super::{Anime, AnimeProvider, Episode, Language, StreamInfo};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;

const CONSUMET_API: &str = "https://api-anime-rouge.vercel.app/gogoanime";

pub struct GogoanimeProvider {
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GogoSearchResult {
    id: String,
    name: String,
    #[serde(default)]
    img: String,
    #[serde(default, rename = "releasedYear")]
    released_year: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GogoEpisode {
    #[serde(rename = "episodeId")]
    episode_id: String,
    #[serde(rename = "episodeNo")]
    episode_no: u32,
}

#[derive(Debug, Deserialize)]
struct GogoStreamSource {
    url: String,
    quality: String,
}

#[derive(Debug, Deserialize)]
struct GogoStreamResponse {
    sources: Vec<GogoStreamSource>,
    headers: Option<serde_json::Value>,
}

impl Default for GogoanimeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GogoanimeProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }
}

#[async_trait]
impl AnimeProvider for GogoanimeProvider {
    fn name(&self) -> &str {
        "Gogoanime"
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
            .context("Failed to search Gogoanime")?
            .json()
            .await
            .context("Failed to parse search response")?;

        let mut results = Vec::new();
        if let Some(animes) = response["animes"].as_array() {
            for result in animes {
                let id = result["id"].as_str().unwrap_or_default().to_string();
                let name = result["name"].as_str().unwrap_or_default().to_string();
                let img = result["img"].as_str().unwrap_or_default();
                
                let img_url = if img.starts_with("http") {
                    img.to_string()
                } else {
                    format!("https://gogocdn.net{}", img)
                };
                
                if !id.is_empty() && !name.is_empty() {
                    results.push(Anime {
                        id,
                        provider: "Gogoanime".to_string(),
                        title: name,
                        cover_url: img_url,
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
        let url = format!("{}/anime/{}", CONSUMET_API, anime_id);

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
        if let Ok(eps) = serde_json::from_value::<Vec<GogoEpisode>>(response["episodes"].clone()) {
            for ep in eps {
                episodes.push(Episode {
                    number: ep.episode_no,
                    title: None,
                    thumbnail: None,
                });
            }
        }

        Ok(episodes)
    }

    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo> {
        let parts: Vec<&str> = episode_id.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid episode_id format");
        }

        let anime_id = parts[0];
        let ep_number = parts[1];

        let url = format!("{}/servers/{}/{}", CONSUMET_API, anime_id, ep_number);
        let servers: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get servers")?
            .json()
            .await
            .context("Failed to parse servers")?;

        let server_id = servers[0]["serverId"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No servers found"))?;

        let watch_url = format!("{}/watch/{}/{}/{}", CONSUMET_API, anime_id, ep_number, server_id);
        let stream_resp: GogoStreamResponse = self
            .client
            .get(&watch_url)
            .send()
            .await
            .context("Failed to get stream URL")?
            .json()
            .await
            .context("Failed to parse stream response")?;

        let source = stream_resp
            .sources
            .first()
            .ok_or_else(|| anyhow::anyhow!("No stream sources found"))?;

        let qualities: Vec<String> = stream_resp
            .sources
            .iter()
            .map(|s| s.quality.clone())
            .collect();

        let mut headers = HashMap::new();
        if let Some(hdrs) = stream_resp.headers {
            if let Some(referer) = hdrs["Referer"].as_str() {
                headers.insert("Referer".to_string(), referer.to_string());
            }
        }

        Ok(StreamInfo {
            video_url: source.url.clone(),
            subtitles: vec![],
            qualities,
            headers,
        })
    }
}
