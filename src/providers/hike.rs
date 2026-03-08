use super::{Anime, AnimeProvider, Episode, Language, StreamInfo};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;

const HIKE_API: &str = "https://anime-kh-hianime.vercel.app/api/v2";

pub struct HikeProvider {
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    data: SearchData,
}

#[derive(Debug, Deserialize, Default)]
struct SearchData {
    #[serde(default)]
    animes: Vec<AnimeResult>,
}

#[derive(Debug, Deserialize)]
struct AnimeResult {
    id: String,
    name: String,
    #[serde(default)]
    poster: String,
}

#[derive(Debug, Deserialize)]
struct AnimeInfoResponse {
    #[serde(default)]
    data: AnimeInfo,
}

#[derive(Debug, Deserialize, Default)]
struct AnimeInfo {
    anime: AnimeDetail,
}

#[derive(Debug, Deserialize, Default)]
struct AnimeDetail {
    #[allow(dead_code)]
    #[serde(default)]
    info: AnimeInfoDetail,
    #[serde(default)]
    episodes: Vec<EpisodeInfo>,
}

#[derive(Debug, Deserialize, Default)]
struct AnimeInfoDetail {
    #[allow(dead_code)]
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize, Default)]
struct EpisodeInfo {
    #[allow(dead_code)]
    id: String,
    number: u32,
    #[serde(default)]
    title: String,
}

#[derive(Debug, Deserialize)]
struct EpisodeSourceResponse {
    #[serde(default)]
    data: EpisodeSource,
}

#[derive(Debug, Deserialize, Default)]
struct EpisodeSource {
    episode: EpisodeSourceDetail,
}

#[derive(Debug, Deserialize, Default)]
struct EpisodeSourceDetail {
    servers: Vec<Server>,
}

#[derive(Debug, Deserialize, Default)]
struct Server {
    #[allow(dead_code)]
    server_id: String,
    server_name: String,
}

impl Default for HikeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl HikeProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }
}

#[async_trait]
impl AnimeProvider for HikeProvider {
    fn name(&self) -> &str {
        "HiAnime"
    }

    fn language(&self) -> Language {
        Language::Vietnamese
    }

    async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        let url = format!("{}/hianime/search", HIKE_API);

        let response: SearchResponse = self
            .client
            .get(&url)
            .query(&[("q", query), ("page", "1")])
            .send()
            .await
            .context("Failed to search HiAnime")?
            .json()
            .await
            .context("Failed to parse search response")?;

        let results: Vec<Anime> = response
            .data
            .animes
            .into_iter()
            .map(|anime| Anime {
                id: anime.id.clone(),
                provider: "HiAnime".to_string(),
                title: anime.name,
                cover_url: anime.poster,
                language: Language::Vietnamese,
                total_episodes: None, // Will be populated when getting info
                synopsis: None,
            })
            .collect();

        Ok(results)
    }

    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        let url = format!("{}/hianime/anime/{}?episodes=true", HIKE_API, anime_id);

        let response: AnimeInfoResponse = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get anime info")?
            .json()
            .await
            .context("Failed to parse anime info")?;

        let episodes: Vec<Episode> = response
            .data
            .anime
            .episodes
            .into_iter()
            .map(|ep| Episode {
                number: ep.number,
                title: if ep.title.is_empty() {
                    None
                } else {
                    Some(ep.title)
                },
                thumbnail: None,
            })
            .collect();

        Ok(episodes)
    }

    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo> {
        // First get available servers
        let servers_url = format!(
            "{}/hianime/episode/servers?animeEpisodeId={}",
            HIKE_API, episode_id
        );

        let servers_response: EpisodeSourceResponse = self
            .client
            .get(&servers_url)
            .send()
            .await
            .context("Failed to get servers")?
            .json()
            .await
            .context("Failed to parse servers response")?;

        // Try to get source from first available server
        if let Some(server) = servers_response.data.episode.servers.first() {
            let source_url = format!(
                "{}/hianime/episode/sources?animeEpisodeId={}&server={}&category=raw",
                HIKE_API, episode_id, server.server_name
            );

            let source_response = self
                .client
                .get(&source_url)
                .send()
                .await
                .context("Failed to get sources")?
                .json::<serde_json::Value>()
                .await
                .context("Failed to parse sources response")?;

            // Extract video URL from response
            if let Some(tracks) = source_response["data"]["tracks"].as_array() {
                for track in tracks {
                    if let Some(file) = track["file"].as_str() {
                        if file.contains(".m3u8") || file.contains(".mp4") {
                            let mut headers = std::collections::HashMap::new();
                            headers.insert("Referer".to_string(), "https://hianime.to".to_string());

                            return Ok(StreamInfo {
                                video_url: file.to_string(),
                                subtitles: vec![],
                                qualities: vec!["auto".to_string()],
                                headers,
                            });
                        }
                    }
                }
            }

            // Try intro/ending sources
            if let Some(intro) = source_response["data"]["intro"]["file"].as_str() {
                let mut headers = std::collections::HashMap::new();
                headers.insert("Referer".to_string(), "https://hianime.to".to_string());

                return Ok(StreamInfo {
                    video_url: intro.to_string(),
                    subtitles: vec![],
                    qualities: vec!["auto".to_string()],
                    headers,
                });
            }
        }

        anyhow::bail!("No stream URL found for this episode")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "HiAnime API is currently unavailable"]
    async fn test_hike_search() {
        let provider = HikeProvider::new();
        let results = provider.search("jujutsu kaisen").await;
        assert!(results.is_ok());
    }
}
