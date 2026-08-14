use super::{Anime, AnimeProvider, Episode, Language, StreamInfo};
use anyhow::{Context, Result};
use async_trait::async_trait;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, REFERER, USER_AGENT};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;

const BASE_URL: &str = "https://www.animegg.org";

pub struct AnimeGgProvider {
    client: reqwest::Client,
}

impl Default for AnimeGgProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimeGgProvider {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/124 Safari/537.36",
            ),
        );
        Self {
            client: reqwest::Client::builder()
                .default_headers(headers)
                .timeout(Duration::from_secs(20))
                .build()
                .expect("failed to build AnimeGG client"),
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
            let body = response
                .text()
                .await
                .with_context(|| format!("{operation} returned an unreadable response"))?;
            if status.is_success() {
                return Ok(body);
            }
            last_error = Some(anyhow::anyhow!(
                "PROVIDER_UNAVAILABLE: {operation} returned HTTP {status}"
            ));
        }
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("{operation} request failed")))
    }

    fn absolute_url(value: &str) -> Result<String> {
        if value.starts_with("http://") || value.starts_with("https://") {
            return Ok(value.to_string());
        }
        Ok(Url::parse(BASE_URL)?.join(value)?.to_string())
    }

    fn parse_search(html: &str) -> Result<Vec<Anime>> {
        let pattern = Regex::new(
            r#"(?s)<a\s+href=[\"'](?P<id>/series/[^\"']+)[\"'][^>]*class=[\"'][^\"']*mse[^\"']*[\"'][^>]*>.*?<img[^>]+src=[\"'](?P<cover>[^\"']+)[\"'].*?<h2[^>]*>(?P<title>.*?)</h2>.*?Episodes:\s*(?P<episodes>\d+)"#,
        )?;
        pattern
            .captures_iter(html)
            .map(|capture| {
                Ok(Anime {
                    id: capture["id"].to_string(),
                    provider: "AnimeGG".into(),
                    title: clean_html(&capture["title"]),
                    cover_url: Self::absolute_url(&capture["cover"])?,
                    language: Language::English,
                    total_episodes: capture["episodes"].parse().ok(),
                    synopsis: None,
                    anilist_id: None,
                })
            })
            .collect()
    }

    fn parse_episodes(html: &str) -> Result<Vec<Episode>> {
        let pattern = Regex::new(
            r#"(?s)<a\s+href=[\"'](?P<id>[^\"']+)[\"'][^>]*class=[\"'][^\"']*anm_det_pop[^\"']*[\"'][^>]*>\s*<strong>(?P<label>.*?)</strong>\s*</a>\s*<i[^>]*class=[\"'][^\"']*anititle[^\"']*[\"'][^>]*>(?P<title>.*?)</i>"#,
        )?;
        let number_pattern = Regex::new(r"(?i)(?:episode\s*)?(\d+)(?:\D*)$")?;
        let mut episodes = pattern
            .captures_iter(html)
            .filter_map(|capture| {
                let label = clean_html(&capture["label"]);
                let number = number_pattern
                    .captures(&label)
                    .and_then(|value| value.get(1))?
                    .as_str()
                    .parse::<u32>()
                    .ok()?;
                Some(Episode {
                    id: capture["id"].to_string(),
                    number,
                    title: Some(clean_html(&capture["title"])),
                    thumbnail: None,
                    aniskip_episode_number: Some(number),
                })
            })
            .collect::<Vec<_>>();
        episodes.sort_by_key(|episode| episode.number);
        episodes.dedup_by_key(|episode| episode.number);
        Ok(episodes)
    }

    fn parse_embed_url(html: &str) -> Result<String> {
        let subbed = Regex::new(
            r#"(?s)id=[\"']subbed-Animegg[\"'].*?<iframe[^>]+src=[\"'](?P<url>[^\"']+)[\"']"#,
        )?;
        let capture = subbed
            .captures(html)
            .context("STREAM_NOT_FOUND: AnimeGG returned no English-sub embed")?;
        Self::absolute_url(&capture["url"])
    }

    fn parse_sources(html: &str, embed_url: &str) -> Result<StreamInfo> {
        let pattern = Regex::new(
            r#"\{\s*file\s*:\s*[\"'](?P<url>[^\"']+)[\"']\s*,\s*label\s*:\s*[\"'](?P<label>[^\"']+)[\"']"#,
        )?;
        let mut sources = pattern
            .captures_iter(html)
            .filter_map(|capture| {
                let url = Self::absolute_url(&capture["url"]).ok()?;
                let label = capture["label"].to_string();
                let quality = label
                    .chars()
                    .filter(char::is_ascii_digit)
                    .collect::<String>()
                    .parse::<u32>()
                    .unwrap_or(0);
                Some((quality, label, url))
            })
            .collect::<Vec<_>>();
        sources.sort_by_key(|source| source.0);
        let (_, _, video_url) = sources
            .last()
            .cloned()
            .context("STREAM_NOT_FOUND: AnimeGG embed returned no playable source")?;
        let mut headers = HashMap::new();
        headers.insert(REFERER.as_str().to_string(), embed_url.to_string());
        Ok(StreamInfo {
            video_url,
            subtitles: Vec::new(),
            qualities: sources.into_iter().map(|source| source.1).collect(),
            headers,
            use_curl: false,
        })
    }
}

#[async_trait]
impl AnimeProvider for AnimeGgProvider {
    fn name(&self) -> &str {
        "AnimeGG"
    }

    fn language(&self) -> Language {
        Language::English
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["en".into()]
    }

    async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        let mut url = Url::parse(&format!("{BASE_URL}/search/"))?;
        url.query_pairs_mut().append_pair("q", query);
        Self::parse_search(&self.html(url, "AnimeGG search").await?)
    }

    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        let url = Url::parse(BASE_URL)?.join(anime_id)?;
        Self::parse_episodes(&self.html(url, "AnimeGG episodes").await?)
    }

    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo> {
        let episode_url = Url::parse(BASE_URL)?.join(episode_id)?;
        let episode_html = self.html(episode_url, "AnimeGG episode").await?;
        let embed_url = Self::parse_embed_url(&episode_html)?;
        let embed_html = self.html(Url::parse(&embed_url)?, "AnimeGG embed").await?;
        Self::parse_sources(&embed_html, &embed_url)
    }
}

fn clean_html(value: &str) -> String {
    let without_tags = Regex::new(r"(?s)<[^>]+>")
        .expect("valid HTML tag pattern")
        .replace_all(value, " ");
    without_tags
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_series_and_embed_sources() {
        let search = r#"<a href="/series/one-piece" class="mse"><img src="/images/op.jpg"><h2>One Piece</h2><div>Episodes: 1167</div></a>"#;
        let items = AnimeGgProvider::parse_search(search).unwrap();
        assert_eq!(items[0].title, "One Piece");
        assert_eq!(items[0].total_episodes, Some(1167));

        let embed = r#"var videoSources = [{file: "/play/op-360/video.mp4", label: "360p"},{file: "/play/op-720/video.mp4", label: "720p"}];"#;
        let stream =
            AnimeGgProvider::parse_sources(embed, "https://www.animegg.org/embed/op").unwrap();
        assert!(stream.video_url.contains("op-720"));
        assert_eq!(stream.qualities, vec!["360p", "720p"]);
    }
}
