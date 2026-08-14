use super::{Anime, AnimeProvider, Episode, Language, StreamInfo};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

const API_BASE: &str = "https://api.animapper.net/api/v1";

pub struct NiniyoProvider {
    client: reqwest::Client,
}

impl Default for NiniyoProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl NiniyoProvider {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/124 Safari/537.36",
            ),
        );
        Self {
            client: reqwest::Client::builder()
                .default_headers(headers)
                .timeout(Duration::from_secs(30))
                .build()
                .expect("failed to build Niniyo client"),
        }
    }

    async fn json(&self, request: reqwest::RequestBuilder, operation: &str) -> Result<Value> {
        for attempt in 0..3 {
            let response = request
                .try_clone()
                .context("Niniyo request could not be retried")?
                .send()
                .await
                .with_context(|| format!("{operation} request failed"))?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or((attempt + 1) as u64)
                .min(5);
            let body = response
                .text()
                .await
                .with_context(|| format!("{operation} returned an unreadable response"))?;
            if status.is_success() {
                return serde_json::from_str(&body)
                    .with_context(|| format!("{operation} returned invalid JSON"));
            }
            if attempt < 2
                && (status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error())
            {
                tokio::time::sleep(Duration::from_secs(retry_after)).await;
                continue;
            }
            let message = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|value| value["message"].as_str().map(str::to_string))
                .unwrap_or_else(|| format!("HTTP {status}"));
            anyhow::bail!("PROVIDER_UNAVAILABLE: {operation} failed: {message}");
        }
        unreachable!("Niniyo retry loop always returns")
    }

    async fn metadata(&self, anime_id: &str) -> Result<Value> {
        self.json(
            self.client
                .get(format!("{API_BASE}/metadata"))
                .query(&[("id", anime_id)]),
            "Niniyo metadata",
        )
        .await
    }

    fn anime_from_value(value: &Value) -> Option<Anime> {
        let id = value["id"].as_i64()?.to_string();
        let titles = &value["titles"];
        let title = titles["vi"]
            .as_str()
            .or_else(|| titles["en"].as_str())
            .or_else(|| titles["main"].as_str())
            .or_else(|| titles["user-preferred"].as_str())?
            .to_string();
        let images = &value["images"];
        let cover_url = images["coverXl"]
            .as_str()
            .or_else(|| images["coverLg"].as_str())
            .or_else(|| images["coverMd"].as_str())
            .unwrap_or_default()
            .to_string();
        Some(Anime {
            id,
            provider: "Niniyo".into(),
            title,
            cover_url,
            language: Language::Vietnamese,
            total_episodes: value["episodes"].as_u64().map(|count| count as u32),
            synopsis: value["descriptions"]["vi"]
                .as_str()
                .or_else(|| value["descriptions"]["en"].as_str())
                .map(str::to_string),
            anilist_id: None,
        })
    }

    fn episodes_from_value(value: &Value) -> Vec<Episode> {
        let mut episodes = value["episodes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|episode| {
                let number_text = episode["episodeNumber"].as_str()?;
                let number = number_text.parse::<f64>().ok()?.floor() as u32;
                if number == 0 {
                    return None;
                }
                Some(Episode {
                    id: episode["episodeId"].as_str()?.to_string(),
                    number,
                    title: Some(format!("Tập {number_text}")),
                    thumbnail: None,
                    aniskip_episode_number: Some(number),
                })
            })
            .collect::<Vec<_>>();
        episodes.sort_by_key(|episode| episode.number);
        episodes.dedup_by_key(|episode| episode.number);
        episodes
    }

    fn candidate_score(value: &Value, query: &str) -> u8 {
        let query = normalize_title(query);
        value["titles"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(_, title)| title.as_str())
            .map(normalize_title)
            .map(|title| {
                if title == query {
                    3
                } else if title.starts_with(&query) || query.starts_with(&title) {
                    2
                } else if title.contains(&query) || query.contains(&title) {
                    1
                } else {
                    0
                }
            })
            .max()
            .unwrap_or_default()
    }

    fn stream_from_value(value: &Value) -> Result<StreamInfo> {
        let stream_type = value["type"].as_str().unwrap_or_default();
        anyhow::ensure!(
            stream_type.eq_ignore_ascii_case("HLS"),
            "STREAM_NOT_FOUND: Niniyo returned {stream_type} instead of direct HLS"
        );
        let video_url = value["url"]
            .as_str()
            .filter(|url| !url.is_empty())
            .context("STREAM_NOT_FOUND: Niniyo returned no stream URL")?
            .to_string();
        let headers = value["proxyHeaders"]
            .as_object()
            .map(|headers| {
                headers
                    .iter()
                    .filter_map(|(name, value)| {
                        value
                            .as_str()
                            .map(|value| (name.clone(), value.to_string()))
                    })
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();
        Ok(StreamInfo {
            video_url,
            subtitles: Vec::new(),
            qualities: vec![value["server"].as_str().unwrap_or("auto").to_string()],
            headers,
            use_curl: false,
        })
    }
}

#[async_trait]
impl AnimeProvider for NiniyoProvider {
    fn name(&self) -> &str {
        "Niniyo"
    }

    fn language(&self) -> Language {
        Language::Vietnamese
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["vi".into()]
    }

    async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        let value = self
            .json(
                self.client.get(format!("{API_BASE}/search")).query(&[
                    ("title", query),
                    ("mediaType", "ANIME"),
                    ("limit", "6"),
                ]),
                "Niniyo search",
            )
            .await?;
        let mut candidates = value["results"].as_array().cloned().unwrap_or_default();
        candidates
            .sort_by_key(|candidate| std::cmp::Reverse(Self::candidate_score(candidate, query)));

        let mut results = Vec::new();
        let mut last_error = None;
        for candidate in candidates.iter().take(3) {
            let Some(anime_id) = candidate["id"].as_i64().map(|id| id.to_string()) else {
                continue;
            };
            let metadata = match self.metadata(&anime_id).await {
                Ok(metadata) => metadata,
                Err(error) => {
                    last_error = Some(error);
                    continue;
                }
            };
            let result = &metadata["result"];
            if result["streamingProviders"]["NINIYO"].is_null() {
                continue;
            }
            if let Some(anime) = Self::anime_from_value(result) {
                results.push(anime);
            }
        }
        if results.is_empty() {
            if let Some(error) = last_error {
                return Err(error.context("Niniyo could not verify provider mappings"));
            }
        }
        Ok(results)
    }

    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        let value = self
            .json(
                self.client
                    .get(format!("{API_BASE}/stream/episodes"))
                    .query(&[("id", anime_id), ("provider", "NINIYO")]),
                "Niniyo episodes",
            )
            .await?;
        let episodes = Self::episodes_from_value(&value);
        anyhow::ensure!(
            !episodes.is_empty(),
            "PROVIDER_UNAVAILABLE: Niniyo has no mapped episodes for this title"
        );
        Ok(episodes)
    }

    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo> {
        let value = self
            .json(
                self.client
                    .get(format!("{API_BASE}/stream/source"))
                    .query(&[
                        ("episodeData", episode_id),
                        ("provider", "NINIYO"),
                        ("server", "DU"),
                    ]),
                "Niniyo stream",
            )
            .await?;
        Self::stream_from_value(&value)
    }
}

fn normalize_title(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_search_episodes_and_hls_source() {
        let search = serde_json::json!({
            "id": 151807,
            "titles": { "vi": "Thăng Cấp Một Mình", "en": "Solo Leveling" },
            "images": { "coverXl": "https://img.example/cover.png" }
        });
        let anime = NiniyoProvider::anime_from_value(&search).unwrap();
        assert_eq!(anime.id, "151807");
        assert_eq!(anime.title, "Thăng Cấp Một Mình");

        let episodes = NiniyoProvider::episodes_from_value(&serde_json::json!({
            "episodes": [
                { "episodeNumber": "12", "episodeId": "solo-leveling$12" },
                { "episodeNumber": "1", "episodeId": "solo-leveling$1" }
            ]
        }));
        assert_eq!(episodes[0].number, 1);
        assert_eq!(episodes[1].id, "solo-leveling$12");

        let stream = NiniyoProvider::stream_from_value(&serde_json::json!({
            "server": "P16",
            "type": "HLS",
            "url": "https://media.example/episode.m3u8",
            "proxyHeaders": { "Referer": "https://niniyo.com" }
        }))
        .unwrap();
        assert_eq!(stream.video_url, "https://media.example/episode.m3u8");
        assert_eq!(stream.headers["Referer"], "https://niniyo.com");
    }

    #[test]
    fn rejects_embed_only_source() {
        let result = NiniyoProvider::stream_from_value(&serde_json::json!({
            "type": "EMBED",
            "url": "https://player.example/embed"
        }));
        assert!(result.is_err());
    }

    #[test]
    fn title_scoring_prefers_exact_aliases() {
        let exact = serde_json::json!({ "titles": { "en": "Attack on Titan" } });
        let sequel = serde_json::json!({ "titles": { "en": "Attack on Titan Season 3" } });
        let unrelated = serde_json::json!({ "titles": { "en": "Titan A.E." } });
        assert_eq!(
            NiniyoProvider::candidate_score(&exact, "Attack on Titan"),
            3
        );
        assert_eq!(
            NiniyoProvider::candidate_score(&sequel, "Attack on Titan"),
            2
        );
        assert_eq!(
            NiniyoProvider::candidate_score(&unrelated, "Attack on Titan"),
            0
        );
    }
}
