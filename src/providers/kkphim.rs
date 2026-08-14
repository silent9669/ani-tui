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

    fn absolute_image_url(cdn: &str, value: &str) -> Option<String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed.starts_with("http") {
            Some(trimmed.to_string())
        } else {
            Some(format!(
                "{}/{}",
                cdn.trim_end_matches('/'),
                trimmed.trim_start_matches('/')
            ))
        }
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
                let mut items = items.clone();
                // Sort items to prioritize anime (type: "hoathinh")
                items.sort_by(|a, b| {
                    let a_type = a["type"].as_str().unwrap_or("");
                    let b_type = b["type"].as_str().unwrap_or("");
                    let a_priority = if a_type == "hoathinh" { 0 } else { 1 };
                    let b_priority = if b_type == "hoathinh" { 0 } else { 1 };
                    a_priority.cmp(&b_priority)
                });

                for item in items {
                    let slug = item["slug"].as_str().unwrap_or_default().to_string();
                    let name = item["name"].as_str().unwrap_or_default().to_string();

                    let thumb = item["thumb_url"].as_str().unwrap_or_default();
                    let poster = item["poster_url"].as_str().unwrap_or_default();

                    let cdn = response["data"]["APP_DOMAIN_CDN_IMAGE"]
                        .as_str()
                        .unwrap_or("https://phimimg.com");

                    let image_url = Self::absolute_image_url(cdn, thumb)
                        .or_else(|| Self::absolute_image_url(cdn, poster))
                        .unwrap_or_default();

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
                            anilist_id: None,
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        let detail_url = format!("{}/phim/{}?embed=false", KKPHIM_API, anime_id);

        let response: serde_json::Value = self
            .client
            .get(&detail_url)
            .send()
            .await
            .context("Failed to get KKPhim episodes")?
            .json()
            .await
            .context("Failed to parse KKPhim episodes response")?;

        let mut episodes = Vec::new();

        if let Some(data) = response.get("data") {
            if let Some(item) = data.get("item") {
                if let Some(episode_list) = item.get("episodes").and_then(|e| e.as_array()) {
                    for server in episode_list {
                        if let Some(server_data) =
                            server.get("server_data").and_then(|s| s.as_array())
                        {
                            for ep in server_data {
                                // Parse Vietnamese episode name: "Tập 001" -> 1
                                let name_str = ep["name"].as_str().unwrap_or("");
                                let ep_number = super::parse_episode_number(name_str);

                                if ep_number > 0 {
                                    episodes.push(Episode {
                                        id: format!("{}:{}", anime_id, ep_number),
                                        number: ep_number,
                                        title: Some(format!("Episode {}", ep_number)),
                                        thumbnail: None,
                                        aniskip_episode_number: Some(ep_number),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        episodes.sort_by_key(|a| a.number);
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

        let mut stream_url = String::new();
        let mut subtitles: Vec<Subtitle> = Vec::new();
        let mut headers: HashMap<String, String> = HashMap::new();

        if let Some(data) = response.get("data") {
            if let Some(item) = data.get("item") {
                if let Some(episode_list) = item.get("episodes").and_then(|e| e.as_array()) {
                    // Sort servers to prioritize Vietsub (usually "#Hà Nội")
                    let mut sorted_servers = episode_list.clone();
                    sorted_servers.sort_by(|a, b| {
                        let a_name = a["server_name"].as_str().unwrap_or("").to_lowercase();
                        let b_name = b["server_name"].as_str().unwrap_or("").to_lowercase();
                        let a_priority = if a_name.contains("hà nội") || a_name.contains("vietsub")
                        {
                            0
                        } else {
                            1
                        };
                        let b_priority = if b_name.contains("hà nội") || b_name.contains("vietsub")
                        {
                            0
                        } else {
                            1
                        };
                        a_priority.cmp(&b_priority)
                    });

                    'outer: for server in sorted_servers {
                        if let Some(server_data) =
                            server.get("server_data").and_then(|s| s.as_array())
                        {
                            for server_ep in server_data {
                                let ep_name = server_ep["name"].as_str().unwrap_or("");
                                // Parse Vietnamese episode name: "Tập 001" -> 1
                                let ep_num = super::parse_episode_number(ep_name);
                                let search_num = episode_number.parse::<u32>().unwrap_or(0);

                                if ep_num == search_num {
                                    if let Some(link) = server_ep["link_m3u8"].as_str() {
                                        if !link.is_empty() {
                                            stream_url = link.to_string();
                                        }
                                    }

                                    if stream_url.is_empty() {
                                        if let Some(link) = server_ep["link_embed"].as_str() {
                                            if link.contains("url=") {
                                                if let Some(url_part) = link.split("url=").last() {
                                                    stream_url = url_part.to_string();
                                                }
                                            } else {
                                                stream_url = link.to_string();
                                            }
                                        }
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
                }
            }
        }

        if stream_url.is_empty() {
            anyhow::bail!("No working stream URL found for this episode.");
        }

        headers.insert("User-Agent".to_string(), "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string());
        headers.insert("Referer".to_string(), "https://phimmoiii.net/".to_string());
        headers.insert("Origin".to_string(), "https://phimmoiii.net".to_string());

        Ok(StreamInfo {
            video_url: stream_url,
            subtitles,
            qualities: vec!["auto".to_string()],
            headers,
            use_curl: false,
        })
    }
}
