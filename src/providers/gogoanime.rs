use super::{Anime, AnimeProvider, Episode, Language, StreamInfo};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;

const CONSUMET_API: &str = "https://api.consumet.org";

pub struct GogoanimeProvider {
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GogoSearchResult {
    id: String,
    title: String,
    #[serde(default)]
    image: String,
    #[serde(default, rename = "releaseDate")]
    release_date: Option<String>,
    #[serde(default, rename = "subOrDub")]
    sub_or_dub: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GogoEpisode {
    id: String,
    number: u32,
}

#[derive(Debug, Deserialize)]
struct GogoStreamSource {
    url: String,
    quality: String,
}

#[derive(Debug, Deserialize)]
struct GogoStreamResponse {
    headers: Option<serde_json::Value>,
    sources: Vec<GogoStreamSource>,
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

    /// Decode ani-cli style provider ID
    #[allow(dead_code)]
    fn decode_provider_id(encoded: &str) -> String {
        // Remove -- prefix if present
        let encoded = encoded.trim_start_matches("--");

        // Map hex pairs to characters (same as ani-cli)
        let mapping: HashMap<&str, char> = [
            ("79", 'A'),
            ("7a", 'B'),
            ("7b", 'C'),
            ("7c", 'D'),
            ("7d", 'E'),
            ("7e", 'F'),
            ("7f", 'G'),
            ("70", 'H'),
            ("71", 'I'),
            ("72", 'J'),
            ("73", 'K'),
            ("74", 'L'),
            ("75", 'M'),
            ("76", 'N'),
            ("77", 'O'),
            ("68", 'P'),
            ("69", 'Q'),
            ("6a", 'R'),
            ("6b", 'S'),
            ("6c", 'T'),
            ("6d", 'U'),
            ("6e", 'V'),
            ("6f", 'W'),
            ("60", 'X'),
            ("61", 'Y'),
            ("62", 'Z'),
            ("59", 'a'),
            ("5a", 'b'),
            ("5b", 'c'),
            ("5c", 'd'),
            ("5d", 'e'),
            ("5e", 'f'),
            ("5f", 'g'),
            ("50", 'h'),
            ("51", 'i'),
            ("52", 'j'),
            ("53", 'k'),
            ("54", 'l'),
            ("55", 'm'),
            ("56", 'n'),
            ("57", 'o'),
            ("48", 'p'),
            ("49", 'q'),
            ("4a", 'r'),
            ("4b", 's'),
            ("4c", 't'),
            ("4d", 'u'),
            ("4e", 'v'),
            ("4f", 'w'),
            ("40", 'x'),
            ("41", 'y'),
            ("42", 'z'),
            ("08", '0'),
            ("09", '1'),
            ("0a", '2'),
            ("0b", '3'),
            ("0c", '4'),
            ("0d", '5'),
            ("0e", '6'),
            ("0f", '7'),
            ("00", '8'),
            ("01", '9'),
            ("15", '-'),
            ("16", '.'),
            ("67", '_'),
            ("46", '~'),
            ("02", ':'),
            ("17", '/'),
            ("07", '?'),
            ("1b", '#'),
            ("63", '['),
            ("65", ']'),
            ("78", '@'),
            ("19", '!'),
            ("1c", '$'),
            ("1e", '&'),
            ("10", '('),
            ("11", ')'),
            ("12", '*'),
            ("13", '+'),
            ("14", ','),
            ("03", ';'),
            ("05", '='),
            ("1d", '%'),
        ]
        .iter()
        .cloned()
        .collect();

        let mut result = String::new();
        let chars: Vec<char> = encoded.chars().collect();

        for chunk in chars.chunks(2) {
            if chunk.len() == 2 {
                let hex = format!("{}{}", chunk[0], chunk[1]);
                if let Some(&ch) = mapping.get(hex.as_str()) {
                    result.push(ch);
                }
            }
        }

        // Fix clock.json path
        result.replace("/clock", "/clock.json")
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

    async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        let url = format!("{}/anime/gogoanime/{}", CONSUMET_API, query);

        let results: Vec<GogoSearchResult> = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to search Gogoanime")?
            .json()
            .await
            .context("Failed to parse search response")?;

        let anime_list: Vec<Anime> = results
            .into_iter()
            .map(|result| Anime {
                id: result.id,
                provider: "Gogoanime".to_string(),
                title: result.title,
                cover_url: result.image,
                language: Language::English,
                total_episodes: None,
                synopsis: None,
            })
            .collect();

        Ok(anime_list)
    }

    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        let url = format!("{}/anime/gogoanime/info/{}", CONSUMET_API, anime_id);

        let response: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get anime info")?
            .json()
            .await
            .context("Failed to parse anime info")?;

        let episodes: Vec<Episode> = response["episodes"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|ep| Episode {
                number: ep["number"].as_u64().unwrap_or(0) as u32,
                title: ep["title"].as_str().map(|s| s.to_string()),
                thumbnail: None,
            })
            .collect();

        Ok(episodes)
    }

    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo> {
        // episode_id format: "anime_id:episode_number"
        let parts: Vec<&str> = episode_id.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid episode_id format");
        }

        let anime_id = parts[0];
        let ep_number = parts[1];

        // Get episode list to find the episode ID
        let url = format!("{}/anime/gogoanime/info/{}", CONSUMET_API, anime_id);
        let info: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get anime info")?
            .json()
            .await
            .context("Failed to parse info")?;

        // Find the episode with matching number
        let empty_vec = vec![];
        let episodes = info["episodes"].as_array().unwrap_or(&empty_vec);
        let episode = episodes
            .iter()
            .find(|ep| ep["number"].as_u64().unwrap_or(0).to_string() == ep_number)
            .ok_or_else(|| anyhow::anyhow!("Episode not found"))?;

        let ep_id = episode["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No episode ID"))?;

        // Get streaming links
        let stream_url = format!("{}/anime/gogoanime/watch/{}", CONSUMET_API, ep_id);
        let stream_resp: GogoStreamResponse = self
            .client
            .get(&stream_url)
            .send()
            .await
            .context("Failed to get stream URL")?
            .json()
            .await
            .context("Failed to parse stream response")?;

        // Get best quality source
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
