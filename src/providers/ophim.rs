use super::{Anime, AnimeProvider, Episode, Language, StreamInfo, Subtitle};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::header::{self, HeaderMap};
use std::collections::HashMap;
use std::time::Duration;

const OPHIM_API: &str = "https://ophim1.com/v1/api";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub struct OphimProvider {
    client: reqwest::Client,
}

impl Default for OphimProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OphimProvider {
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
impl AnimeProvider for OphimProvider {
    fn name(&self) -> &str {
        "OPhim"
    }

    fn language(&self) -> Language {
        Language::Vietnamese
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["🇻🇳".to_string()]
    }

    async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        let search_url = format!("{}/tim-kiem", OPHIM_API);

        let response: serde_json::Value = self
            .client
            .get(&search_url)
            .query(&[("keyword", query), ("limit", "40")])
            .send()
            .await
            .context("Failed to search OPhim")?
            .json()
            .await
            .context("Failed to parse OPhim search response")?;

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
                    let thumb = item["thumb_url"].as_str().unwrap_or_default().to_string();
                    let poster = item["poster_url"].as_str().unwrap_or_default().to_string();

                    // Use APP_DOMAIN_CDN_IMAGE if available, or fallback to known CDN
                    let cdn = response["data"]["APP_DOMAIN_CDN_IMAGE"]
                        .as_str()
                        .unwrap_or("https://img.ophim.live");

                    let image_url = if poster.starts_with("http") {
                        poster
                    } else if thumb.starts_with("http") {
                        thumb
                    } else {
                        format!("{}/uploads/movies/{}", cdn.trim_end_matches('/'), poster)
                    };

                    if !slug.is_empty() && !name.is_empty() {
                        results.push(Anime {
                            id: slug,
                            provider: "OPhim".to_string(),
                            title: name,
                            cover_url: image_url,
                            language: Language::Vietnamese,
                            total_episodes: None,
                            synopsis: None,
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        let detail_url = format!("{}/phim/{}", OPHIM_API, anime_id);

        let response: serde_json::Value = self
            .client
            .get(&detail_url)
            .send()
            .await
            .context("Failed to get OPhim episodes")?
            .json()
            .await
            .context("Failed to parse OPhim episodes response")?;

        let mut episodes = Vec::new();

        if let Some(data) = response.get("data") {
            if let Some(item) = data.get("item") {
                if let Some(episode_list) = item.get("episodes").and_then(|e| e.as_array()) {
                    for server in episode_list {
                        if let Some(server_data) =
                            server.get("server_data").and_then(|s| s.as_array())
                        {
                            for ep in server_data {
                                let name = ep["name"].as_str().unwrap_or("");
                                let ep_num = super::parse_episode_number(name);

                                if ep_num > 0 {
                                    episodes.push(Episode {
                                        id: format!("{}:{}", anime_id, ep_num),
                                        number: ep_num,
                                        title: Some(
                                            ep["filename"].as_str().unwrap_or("").to_string(),
                                        ),
                                        thumbnail: None,
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

        let detail_url = format!("{}/phim/{}", OPHIM_API, anime_slug);

        let response: serde_json::Value = self
            .client
            .get(&detail_url)
            .send()
            .await
            .context("Failed to get OPhim stream")?
            .json()
            .await
            .context("Failed to parse OPhim stream response")?;

        let mut stream_url = String::new();
        let mut subtitles: Vec<Subtitle> = Vec::new();

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
                            for ep in server_data {
                                let name = ep["name"].as_str().unwrap_or("");
                                let ep_num = super::parse_episode_number(name);
                                let search_num = episode_number.parse::<u32>().unwrap_or(0);

                                if ep_num == search_num {
                                    if let Some(link) = ep["link_m3u8"].as_str() {
                                        if !link.is_empty() {
                                            stream_url = link.to_string();
                                        }
                                    }

                                    if stream_url.is_empty() {
                                        if let Some(link) = ep["link_embed"].as_str() {
                                            if link.contains("url=") {
                                                if let Some(url_part) = link.split("url=").last() {
                                                    stream_url = url_part.to_string();
                                                }
                                            } else {
                                                stream_url = link.to_string();
                                            }
                                        }
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
        }

        if stream_url.is_empty() {
            anyhow::bail!("No working stream URL found for this episode.");
        }

        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("User-Agent".to_string(), "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string());
        headers.insert("Referer".to_string(), "https://ophim17.cc/".to_string());
        headers.insert("Origin".to_string(), "https://ophim17.cc".to_string());

        Ok(StreamInfo {
            video_url: stream_url,
            subtitles,
            qualities: vec!["auto".to_string()],
            headers,
        })
    }
}
