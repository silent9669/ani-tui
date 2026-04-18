use super::{Anime, AnimeProvider, Episode, Language, StreamInfo, Subtitle};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::header::{self, HeaderMap};
use std::collections::HashMap;
use std::time::Duration;

const NGUONC_API: &str = "https://phim.nguonc.com/api";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub struct NguoncProvider {
    client: reqwest::Client,
}

impl Default for NguoncProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl NguoncProvider {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(header::USER_AGENT, header::HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
        ));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }
}

#[async_trait]
impl AnimeProvider for NguoncProvider {
    fn name(&self) -> &str {
        "NguonC"
    }

    fn language(&self) -> Language {
        Language::Vietnamese
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["🇻🇳".to_string()]
    }

    async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        let search_url = format!("{}/films/search", NGUONC_API);

        let response: serde_json::Value = self
            .client
            .get(&search_url)
            .query(&[("keyword", query)])
            .send()
            .await
            .context("Failed to search NguonC")?
            .json()
            .await
            .context("Failed to parse NguonC search response")?;

        let mut results = Vec::new();

        if let Some(items) = response.get("items").and_then(|i| i.as_array()) {
            for item in items {
                let slug = item["slug"].as_str().unwrap_or_default().to_string();
                let name = item["name"].as_str().unwrap_or_default().to_string();
                let poster = item["poster_url"].as_str().unwrap_or_default().to_string();

                let episode_count =
                    item["total_episodes"]
                        .as_u64()
                        .map(|e| e as u32)
                        .or_else(|| {
                            item["total_episodes"]
                                .as_str()
                                .and_then(|e| e.parse::<u32>().ok())
                        });

                if !slug.is_empty() && !name.is_empty() {
                    results.push(Anime {
                        id: slug,
                        provider: "NguonC".to_string(),
                        title: name,
                        cover_url: poster,
                        language: Language::Vietnamese,
                        total_episodes: episode_count,
                        synopsis: item["description"].as_str().map(|s| s.to_string()),
                    });
                }
            }
        }

        Ok(results)
    }

    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        let detail_url = format!("{}/film/{}", NGUONC_API, anime_id);

        let response: serde_json::Value = self
            .client
            .get(&detail_url)
            .send()
            .await
            .context("Failed to get NguonC episodes")?
            .json()
            .await
            .context("Failed to parse NguonC episodes response")?;

        let mut episodes = Vec::new();

        if let Some(movie) = response.get("movie") {
            if let Some(episode_list) = movie.get("episodes").and_then(|e| e.as_array()) {
                for server in episode_list {
                    if let Some(items) = server.get("items").and_then(|i| i.as_array()) {
                        for ep in items {
                            let name_str = ep["name"].as_str().unwrap_or("");
                            let ep_number = name_str.parse::<u32>().unwrap_or_else(|_| {
                                name_str
                                    .chars()
                                    .filter(|c| c.is_ascii_digit())
                                    .collect::<String>()
                                    .parse::<u32>()
                                    .unwrap_or(0)
                            });

                            if ep_number > 0 {
                                episodes.push(Episode {
                                    number: ep_number,
                                    title: Some(format!("Episode {}", ep_number)),
                                    thumbnail: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        episodes.sort_by(|a, b| a.number.cmp(&b.number));
        episodes.dedup_by(|a, b| a.number == b.number);

        Ok(episodes)
    }

    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo> {
        let parts: Vec<&str> = episode_id.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid episode_id format. Expected 'anime_slug:episode_number'");
        }

        let anime_slug = parts[0];
        let episode_number = parts[1];

        let detail_url = format!("{}/film/{}", NGUONC_API, anime_slug);

        let response: serde_json::Value = self
            .client
            .get(&detail_url)
            .send()
            .await
            .context("Failed to get NguonC stream")?
            .json()
            .await
            .context("Failed to parse NguonC stream response")?;

        let mut stream_url = String::new();
        let mut subtitles: Vec<Subtitle> = Vec::new();

        if let Some(movie) = response.get("movie") {
            if let Some(episode_list) = movie.get("episodes").and_then(|e| e.as_array()) {
                'outer: for server in episode_list {
                    if let Some(items) = server.get("items").and_then(|i| i.as_array()) {
                        for ep in items {
                            let ep_name = ep["name"].as_str().unwrap_or("");
                            let ep_num = ep_name.parse::<u32>().unwrap_or_else(|_| {
                                ep_name
                                    .chars()
                                    .filter(|c| c.is_ascii_digit())
                                    .collect::<String>()
                                    .parse::<u32>()
                                    .unwrap_or(0)
                            });
                            let search_num = episode_number.parse::<u32>().unwrap_or(0);

                            if ep_num == search_num {
                                if let Some(link) = ep["m3u8"].as_str() {
                                    stream_url = link.to_string();
                                } else if let Some(link) = ep["embed"].as_str() {
                                    stream_url = link.to_string();
                                }

                                subtitles.push(Subtitle {
                                    language: "vi".to_string(),
                                    url: String::new(),
                                });

                                break 'outer;
                            }
                        }
                    }
                }
            }
        }

        if stream_url.is_empty() {
            anyhow::bail!("No working stream URL found for this episode.");
        }

        Ok(StreamInfo {
            video_url: stream_url,
            subtitles,
            qualities: vec!["auto".to_string()],
            headers: HashMap::new(),
        })
    }
}
