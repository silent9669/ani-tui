use super::{Anime, AnimeProvider, Episode, Language, StreamInfo, Subtitle};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::header::{self, HeaderMap};
use std::collections::HashMap;
use std::time::Duration;

const KKPHIM_API: &str = "https://phimapi.com/v1/api";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub struct KkphimProvider {
    client: reqwest::Client,
}

impl Default for KkphimProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl KkphimProvider {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(header::USER_AGENT, header::HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
        ));
        headers.insert(
            header::REFERER,
            header::HeaderValue::from_static("https://phimmoiii.net/"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }
}

#[async_trait]
impl AnimeProvider for KkphimProvider {
    fn name(&self) -> &str {
        "KKPhim"
    }

    fn language(&self) -> Language {
        Language::Vietnamese
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["🇻🇳".to_string()]
    }

    async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        let search_url = format!("{}/tim-kiem", KKPHIM_API);

        let response: serde_json::Value = self
            .client
            .get(&search_url)
            .query(&[("keyword", query), ("limit", "40")])
            .send()
            .await
            .context("Failed to search KKPhim")?
            .json()
            .await
            .context("Failed to parse KKPhim search response")?;

        let mut results = Vec::new();

        if let Some(data) = response.get("data") {
            if let Some(items) = data.get("items").and_then(|i| i.as_array()) {
                for item in items {
                    let slug = item["slug"].as_str().unwrap_or_default().to_string();
                    let name = item["name"].as_str().unwrap_or_default().to_string();

                    let thumb = item["thumb_url"].as_str().unwrap_or_default();
                    let poster = item["poster_url"].as_str().unwrap_or_default();

                    let cdn = response["data"]["APP_DOMAIN_CDN_IMAGE"]
                        .as_str()
                        .unwrap_or("https://phimimg.com");

                    let image_url = if thumb.starts_with("http") {
                        thumb.to_string()
                    } else if poster.starts_with("http") {
                        poster.to_string()
                    } else {
                        format!("{}/{}", cdn.trim_end_matches('/'), thumb)
                    };

                    let episode_count = item["episode_total"]
                        .as_str()
                        .and_then(|e| e.parse::<u32>().ok());

                    if !slug.is_empty() && !name.is_empty() {
                        results.push(Anime {
                            id: slug,
                            provider: "KKPhim".to_string(),
                            title: name,
                            cover_url: image_url,
                            language: Language::Vietnamese,
                            total_episodes: episode_count,
                            synopsis: item["content"].as_str().map(|s| s.to_string()),
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        let detail_url = format!("{}/phim/{}?embed=false", KKPHIM_API, anime_id);

        tracing::info!("Fetching episodes from KKPhim: {}", detail_url);

        let response: serde_json::Value = self
            .client
            .get(&detail_url)
            .send()
            .await
            .context("Failed to get KKPhim episodes")?
            .json()
            .await
            .context("Failed to parse KKPhim episodes response")?;

        // Debug: Log the response structure
        tracing::debug!("KKPhim episodes response: {:?}", response);

        let mut episodes = Vec::new();

        if let Some(data) = response.get("data") {
            // Log item structure for debugging
            if let Some(item) = data.get("item") {
                tracing::debug!(
                    "KKPhim item keys: {:?}",
                    item.as_object().map(|o| o.keys().collect::<Vec<_>>())
                );

                // Try to get episode count from item.episode_total first
                let episode_total = item["episode_total"]
                    .as_str()
                    .and_then(|e| e.parse::<u32>().ok());
                tracing::info!("KKPhim episode_total: {:?}", episode_total);

                if let Some(episode_list) = item.get("episodes").and_then(|e| e.as_array()) {
                    tracing::info!(
                        "Found {} episode server entries in KKPhim",
                        episode_list.len()
                    );

                    for (server_idx, server) in episode_list.iter().enumerate() {
                        let server_name = server["server_name"].as_str().unwrap_or("Unknown");
                        tracing::debug!("Processing server {}: {}", server_idx, server_name);

                        if let Some(server_data) =
                            server.get("server_data").and_then(|s| s.as_array())
                        {
                            tracing::debug!(
                                "Server {} has {} episodes",
                                server_idx,
                                server_data.len()
                            );

                            for ep in server_data {
                                // Parse Vietnamese episode name: "Tập 001" -> 1
                                let name_str = ep["name"].as_str().unwrap_or("");
                                let ep_number = name_str
                                    .replace("Tập ", "")
                                    .replace("Tap ", "")
                                    .trim()
                                    .parse::<u32>()
                                    .unwrap_or_else(|_| {
                                        // Try extracting just numbers as fallback
                                        name_str
                                            .chars()
                                            .filter(|c| c.is_ascii_digit())
                                            .collect::<String>()
                                            .parse::<u32>()
                                            .unwrap_or(0)
                                    });

                                if ep_number > 0 {
                                    let ep_slug = ep["slug"].as_str().unwrap_or_default();
                                    tracing::debug!(
                                        "Adding episode {} (name: {}, slug: {})",
                                        ep_number,
                                        name_str,
                                        ep_slug
                                    );

                                    episodes.push(Episode {
                                        number: ep_number,
                                        title: Some(format!("Episode {}", ep_number)),
                                        thumbnail: None,
                                    });
                                }
                            }
                        } else {
                            tracing::warn!("Server {} has no server_data", server_idx);
                        }
                    }
                } else {
                    tracing::warn!("No episodes array found in KKPhim response");
                }
            } else {
                tracing::warn!("No item found in KKPhim data");
            }
        } else {
            tracing::warn!("No data found in KKPhim response");
        }

        let before_dedup = episodes.len();
        episodes.sort_by(|a, b| a.number.cmp(&b.number));
        episodes.dedup_by(|a, b| a.number == b.number);
        let after_dedup = episodes.len();

        tracing::info!(
            "KKPhim returned {} episodes ({} after deduplication)",
            before_dedup,
            after_dedup
        );

        Ok(episodes)
    }

    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo> {
        let parts: Vec<&str> = episode_id.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid episode_id format. Expected 'anime_slug:episode_number'");
        }

        let anime_slug = parts[0];
        let episode_number = parts[1];

        tracing::info!(
            "Fetching stream URL for KKPhim anime: {}, episode: {}",
            anime_slug,
            episode_number
        );

        let detail_url = format!("{}/phim/{}?embed=false", KKPHIM_API, anime_slug);

        let response: serde_json::Value = self
            .client
            .get(&detail_url)
            .send()
            .await
            .context("Failed to get KKPhim stream")?
            .json()
            .await
            .context("Failed to parse KKPhim stream response")?;

        tracing::debug!(
            "KKPhim stream response structure: {:?}",
            response
                .get("data")
                .and_then(|d| d.get("item"))
                .map(|i| i.as_object().map(|o| o.keys().collect::<Vec<_>>()))
        );

        let mut stream_url = String::new();
        let mut subtitles: Vec<Subtitle> = Vec::new();
        let qualities = vec!["auto".to_string()];
        let mut headers: HashMap<String, String> = HashMap::new();

        if let Some(data) = response.get("data") {
            if let Some(item) = data.get("item") {
                if let Some(episode_list) = item.get("episodes").and_then(|e| e.as_array()) {
                    // Sort servers to prioritize Vietsub (usually "#Hà Nội")
                    let mut sorted_servers = episode_list.clone();
                    sorted_servers.sort_by(|a, b| {
                        let a_name = a["server_name"].as_str().unwrap_or("").to_lowercase();
                        let b_name = b["server_name"].as_str().unwrap_or("").to_lowercase();
                        let a_priority = if a_name.contains("hà nội") || a_name.contains("vietsub") { 0 } else { 1 };
                        let b_priority = if b_name.contains("hà nội") || b_name.contains("vietsub") { 0 } else { 1 };
                        a_priority.cmp(&b_priority)
                    });

                    tracing::info!(
                        "Searching for episode {} in {} server entries",
                        episode_number,
                        sorted_servers.len()
                    );

                    'outer: for (idx, ep) in sorted_servers.iter().enumerate() {
                        tracing::debug!("Checking episode entry {}: {:?}", idx, ep.get("name"));

                        if let Some(server_data) = ep.get("server_data").and_then(|s| s.as_array())
                        {
                            tracing::debug!(
                                "Episode entry {} has {} server entries",
                                idx,
                                server_data.len()
                            );

                            for server_ep in server_data {
                                let ep_name = server_ep["name"].as_str().unwrap_or("");
                                // Parse Vietnamese episode name: "Tập 001" -> 1
                                let ep_num = ep_name
                                    .replace("Tập ", "")
                                    .replace("Tap ", "")
                                    .trim()
                                    .parse::<u32>()
                                    .unwrap_or_else(|_| {
                                        ep_name
                                            .chars()
                                            .filter(|c| c.is_ascii_digit())
                                            .collect::<String>()
                                            .parse::<u32>()
                                            .unwrap_or(0)
                                    });
                                let search_num = episode_number.parse::<u32>().unwrap_or(0);
                                tracing::debug!(
                                    "Comparing '{}' (parsed: {}) with '{}' (parsed: {})",
                                    ep_name,
                                    ep_num,
                                    episode_number,
                                    search_num
                                );

                                if ep_num == search_num {
                                    tracing::info!("Found matching episode {}", episode_number);

                                    if let Some(link) = server_ep["link_m3u8"].as_str() {
                                        stream_url = link.to_string();
                                        tracing::info!("Found m3u8 stream URL: {}", stream_url);
                                    } else if let Some(link) = server_ep["link_embed"].as_str() {
                                        stream_url = link.to_string();
                                        tracing::info!("Found embed stream URL: {}", stream_url);
                                    } else {
                                        tracing::warn!("No stream URL found in server_ep");
                                    }

                                    // KKPhim provides Vietnamese hardcoded subtitles in the video
                                    subtitles.push(Subtitle {
                                        language: "vi".to_string(),
                                        url: String::new(),
                                    });

                                    break 'outer;
                                }
                            }
                        }
                    }
                } else {
                    tracing::warn!("No episodes array found in KKPhim stream response");
                }
            } else {
                tracing::warn!("No item found in KKPhim data");
            }
        } else {
            tracing::warn!("No data found in KKPhim response");
        }

        if stream_url.is_empty() {
            tracing::error!(
                "No working stream URL found for episode {} of {}",
                episode_number,
                anime_slug
            );
            anyhow::bail!("No working stream URL found for this episode.");
        }

        headers.insert("User-Agent".to_string(), "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string());
        headers.insert("Referer".to_string(), "https://phimmoiii.net/".to_string());
        headers.insert("Origin".to_string(), "https://phimmoiii.net".to_string());

        Ok(StreamInfo {
            video_url: stream_url,
            subtitles,
            qualities,
            headers,
        })
    }
}
