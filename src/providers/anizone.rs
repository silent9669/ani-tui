use super::{Anime, AnimeProvider, Episode, Language, StreamInfo, Subtitle};
use anyhow::{Context, Result};
use async_trait::async_trait;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, REFERER, USER_AGENT};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;

const BASE_URL: &str = "https://anizone.to";
const USER_AGENT_VALUE: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

pub struct AniZoneProvider {
    client: reqwest::Client,
}

impl Default for AniZoneProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AniZoneProvider {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
        Self {
            client: reqwest::Client::builder()
                .default_headers(headers)
                .redirect(reqwest::redirect::Policy::limited(8))
                .timeout(Duration::from_secs(60))
                .build()
                .expect("failed to build AniZone client"),
        }
    }

    async fn html(&self, url: Url, operation: &str) -> Result<String> {
        let mut last_error = None;
        for _ in 0..2 {
            let response = match self.client.get(url.clone()).send().await {
                Ok(response) => response,
                Err(error) => {
                    last_error = Some(
                        anyhow::Error::new(error).context(format!("{operation} request failed")),
                    );
                    continue;
                }
            };
            let status = response.status();
            let body = match response.text().await {
                Ok(body) => body,
                Err(error) => {
                    last_error = Some(
                        anyhow::Error::new(error)
                            .context(format!("{operation} returned an unreadable response")),
                    );
                    continue;
                }
            };
            anyhow::ensure!(
                status.is_success(),
                "PROVIDER_UNAVAILABLE: {operation} returned HTTP {status}"
            );
            anyhow::ensure!(
                !body.contains("Just a moment") && !body.contains("cf-chl-"),
                "PROVIDER_CAPTCHA: AniZone returned a browser challenge"
            );
            return Ok(body);
        }
        Err(last_error
            .unwrap_or_else(|| anyhow::anyhow!("{operation} request failed after retries")))
    }

    fn parse_search(html: &str) -> Result<Vec<Anime>> {
        let card = Regex::new(
            r#"(?s)anmTitles:\s*JSON\.parse\('.*?getTitle\(this\.anmTitles,\s*'(?P<title>[^']+)'\).*?wire:key="a-(?P<id>[A-Za-z0-9_-]+)".*?<img\s+src="(?P<cover>[^"]+)""#,
        )?;
        let episodes = Regex::new(r"(?P<count>[0-9,]+)\s+Eps")?;
        let mut results = Vec::new();
        for capture in card.captures_iter(html) {
            let block_start = capture
                .get(0)
                .map(|value| value.start())
                .unwrap_or_default();
            let following = &html[block_start..html.len().min(block_start + 8_000)];
            let total_episodes = episodes
                .captures(following)
                .and_then(|value| value.name("count"))
                .and_then(|value| value.as_str().replace(',', "").parse().ok());
            results.push(Anime {
                id: capture["id"].to_string(),
                provider: "AniZone".into(),
                title: decode_html(&capture["title"]),
                cover_url: capture["cover"].to_string(),
                language: Language::English,
                total_episodes,
                synopsis: None,
                anilist_id: None,
            });
        }
        Ok(results)
    }

    fn regular_episode_count(html: &str) -> Result<u32> {
        let pattern = Regex::new(
            r#"Showing\s*<span[^>]*>[0-9,]+</span>\s*to\s*<span[^>]*>[0-9,]+</span>\s*of\s*<span[^>]*>(?P<count>[0-9,]+)</span>\s*results"#,
        )?;
        pattern
            .captures(html)
            .and_then(|capture| capture.name("count"))
            .and_then(|value| value.as_str().replace(',', "").parse().ok())
            .context("AniZone details returned no regular episode count")
    }

    fn parse_stream(html: &str) -> Result<StreamInfo> {
        let player_payload =
            Regex::new(r#"(?s)vidstackPlayer\(JSON\.parse\('(?P<payload>.*?)'\)\)"#)?
                .captures(html)
                .and_then(|capture| capture.name("payload"))
                .map(|payload| decode_player_payload(payload.as_str()))
                .transpose()?;

        let video_url = player_payload
            .as_ref()
            .and_then(|payload| payload["src"].as_str())
            .map(str::to_string)
            .or_else(|| {
                Regex::new(r#"<media-player[^>]+src="(?P<url>https?://[^"]+)""#)
                    .ok()?
                    .captures(html)
                    .map(|capture| capture["url"].to_string())
            })
            .context("STREAM_NOT_FOUND: AniZone returned no HLS player")?;

        let mut subtitles = if let Some(payload) = player_payload {
            payload["subtitles"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|track| {
                    let language = track["title"].as_str()?.to_string();
                    if !language.to_ascii_lowercase().starts_with("english") {
                        return None;
                    }
                    Some(Subtitle {
                        language,
                        url: track["file"].as_str()?.to_string(),
                    })
                })
                .collect::<Vec<_>>()
        } else {
            let track = Regex::new(
                r#"<track\s+src=(?:"(?P<quoted>https?://[^"]+)"|(?P<plain>https?://[^\s>]+))[^>]*data-type="ass"[^>]*label="(?P<label>[^"]+)""#,
            )?;
            track
                .captures_iter(html)
                .filter_map(|capture| {
                    let language = decode_html(&capture["label"]);
                    if !language.to_ascii_lowercase().starts_with("english") {
                        return None;
                    }
                    let url = capture
                        .name("quoted")
                        .or_else(|| capture.name("plain"))?
                        .as_str()
                        .to_string();
                    Some(Subtitle { language, url })
                })
                .collect::<Vec<_>>()
        };
        subtitles.sort_by_key(|track| {
            let label = track.language.to_ascii_lowercase();
            if label.contains("full subtitles") {
                0
            } else if label == "english" {
                1
            } else {
                2
            }
        });
        anyhow::ensure!(
            !subtitles.is_empty(),
            "STREAM_NOT_FOUND: AniZone returned no English subtitle track"
        );
        let mut headers = HashMap::new();
        headers.insert(REFERER.as_str().to_string(), format!("{BASE_URL}/"));
        headers.insert(
            USER_AGENT.as_str().to_string(),
            USER_AGENT_VALUE.to_string(),
        );
        Ok(StreamInfo {
            video_url,
            subtitles,
            qualities: vec!["Auto".into()],
            headers,
            use_curl: false,
        })
    }

    fn anime_url(anime_id: &str) -> Result<Url> {
        anyhow::ensure!(
            !anime_id.is_empty()
                && anime_id
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric()
                        || character == '-'
                        || character == '_'),
            "AniZone anime ID is invalid"
        );
        Url::parse(&format!("{BASE_URL}/anime/{anime_id}")).map_err(Into::into)
    }

    fn episode_url(episode_id: &str) -> Result<Url> {
        let (anime_id, episode) = episode_id
            .split_once('/')
            .context("AniZone episode ID is invalid")?;
        Self::anime_url(anime_id)?;
        anyhow::ensure!(
            !episode.is_empty() && episode.chars().all(|character| character.is_ascii_digit()),
            "AniZone episode ID is invalid"
        );
        Url::parse(&format!("{BASE_URL}/anime/{anime_id}/{episode}")).map_err(Into::into)
    }
}

#[async_trait]
impl AnimeProvider for AniZoneProvider {
    fn name(&self) -> &str {
        "AniZone"
    }

    fn language(&self) -> Language {
        Language::English
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["en".into()]
    }

    async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        let mut url = Url::parse(&format!("{BASE_URL}/anime"))?;
        url.query_pairs_mut().append_pair("search", query);
        Self::parse_search(&self.html(url, "AniZone search").await?)
    }

    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        let url = Self::anime_url(anime_id)?;
        let html = self.html(url, "AniZone episodes").await?;
        let count = Self::regular_episode_count(&html)?;
        Ok((1..=count)
            .map(|number| Episode {
                id: format!("{anime_id}/{number}"),
                number,
                title: None,
                thumbnail: None,
                aniskip_episode_number: (number > 0).then_some(number),
            })
            .collect())
    }

    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo> {
        let url = Self::episode_url(episode_id)?;
        Self::parse_stream(&self.html(url, "AniZone playback").await?)
    }
}

fn decode_player_payload(payload: &str) -> Result<serde_json::Value> {
    let javascript_string: String = serde_json::from_str(&format!("\"{payload}\""))
        .context("AniZone returned an invalid player payload")?;
    serde_json::from_str(&javascript_string).context("AniZone returned invalid player JSON")
}

fn decode_html(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_search_episode_count_and_ass_playback() {
        let search = r#"anmTitles: JSON.parse('{}'), get displayAnimeTitle() { return window.getTitle(this.anmTitles, 'One Piece'); } } wire:key="a-uyyyn4kf"><img src="https://anizone.to/images/anime/cover.jpg"><span>1176 Eps</span>"#;
        let results = AniZoneProvider::parse_search(search).unwrap();
        assert_eq!(results[0].id, "uyyyn4kf");
        assert_eq!(results[0].title, "One Piece");

        let details = r#"Showing <span>1</span> to <span>36</span> of <span>1,173</span> results"#;
        assert_eq!(
            AniZoneProvider::regular_episode_count(details).unwrap(),
            1173
        );

        let playback = r#"<media-player src="https://cdn.example/master.m3u8"><track src=https://cdn.example/subtitles/signs_en.ass data-type="ass" kind="subtitles" label="English - Signs/Songs" srclang="en" /><track src=https://cdn.example/subtitles/full_en.ass data-type="ass" kind="subtitles" label="English - Full Subtitles" srclang="en" default />"#;
        let stream = AniZoneProvider::parse_stream(playback).unwrap();
        assert_eq!(stream.video_url, "https://cdn.example/master.m3u8");
        assert_eq!(stream.subtitles[0].language, "English - Full Subtitles");

        let encoded = r#"<div x-data="vidstackPlayer(JSON.parse('{\u0022src\u0022:\u0022https:\\/\\/cdn.example\\/master.m3u8\u0022,\u0022subtitles\u0022:[{\u0022title\u0022:\u0022English\u0022,\u0022format\u0022:\u0022ass\u0022,\u0022file\u0022:\u0022https:\\/\\/cdn.example\\/sub.ass\u0022}]}'))"></div>"#;
        let stream = AniZoneProvider::parse_stream(encoded).unwrap();
        assert_eq!(stream.video_url, "https://cdn.example/master.m3u8");
        assert_eq!(stream.subtitles[0].url, "https://cdn.example/sub.ass");
    }
}
