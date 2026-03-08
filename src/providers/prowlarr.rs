use super::{Anime, AnimeProvider, Episode, Language, StreamInfo};
use crate::config::ProwlarrConfig;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;

pub struct ProwlarrProvider {
    config: Option<ProwlarrConfig>,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    #[serde(rename = "age")]
    _age: u32,
    #[serde(rename = "title")]
    title: String,
    #[serde(rename = "guid")]
    guid: String,
    #[allow(dead_code)]
    #[serde(rename = "infoUrl", default)]
    info_url: Option<String>,
    #[allow(dead_code)]
    #[serde(rename = "downloadUrl", default)]
    download_url: Option<String>,
}

impl ProwlarrProvider {
    pub fn new(config: ProwlarrConfig) -> Self {
        let client = reqwest::Client::new();
        Self {
            config: Some(config),
            client,
        }
    }

    pub fn new_unconfigured() -> Self {
        let client = reqwest::Client::new();
        Self {
            config: None,
            client,
        }
    }

    fn is_configured(&self) -> bool {
        self.config.is_some()
    }

    async fn search_prowlarr(&self, query: &str) -> Result<Vec<SearchResult>> {
        let config = match &self.config {
            Some(c) => c,
            None => {
                anyhow::bail!("Prowlarr is not configured. Please set the prowlarr URL and API key in config.toml.");
            }
        };

        let url = format!("{}/api/v1/search", config.url);

        let params = [
            ("query", query),
            ("indexerIds", &config.indexer.to_string()),
        ];

        let response = self
            .client
            .get(&url)
            .query(&params)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header("X-Api-Key", &config.api_key)
            .send()
            .await
            .context("Failed to search Prowlarr")?;

        if !response.status().is_success() {
            anyhow::bail!("Prowlarr search failed with status: {}", response.status());
        }

        let results: Vec<SearchResult> = response
            .json()
            .await
            .context("Failed to parse Prowlarr search results")?;

        Ok(results)
    }
}

#[async_trait]
impl AnimeProvider for ProwlarrProvider {
    fn name(&self) -> &str {
        "Prowlarr"
    }

    fn language(&self) -> Language {
        Language::Vietnamese
    }

    async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        if !self.is_configured() {
            return Ok(Vec::new());
        }

        let results = match self.search_prowlarr(query).await {
            Ok(r) => r,
            Err(_) => {
                return Ok(Vec::new());
            }
        };

        let anime_list: Vec<Anime> = results
            .into_iter()
            .take(20)
            .map(|result| Anime {
                id: result.guid.clone(),
                provider: "Prowlarr".to_string(),
                title: result.title.clone(),
                cover_url: String::new(),
                language: Language::Vietnamese,
                total_episodes: None,
                synopsis: None,
            })
            .collect();

        Ok(anime_list)
    }

    async fn get_episodes(&self, _anime_id: &str) -> Result<Vec<Episode>> {
        if !self.is_configured() {
            return Ok(vec![]);
        }

        Ok(vec![Episode {
            number: 1,
            title: Some("Episode".to_string()),
            thumbnail: None,
        }])
    }

    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo> {
        if !self.is_configured() {
            anyhow::bail!("Prowlarr is not configured. Please set the prowlarr URL and API key in config.toml.");
        }

        let config = self.config.as_ref().unwrap();
        let url = format!("{}/api/v1/search", config.url);

        let request_body = serde_json::json!({
            "guid": episode_id,
            "indexerId": config.indexer
        });

        let response = self
            .client
            .post(&url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header("X-Api-Key", &config.api_key)
            .json(&request_body)
            .send()
            .await
            .context("Failed to initiate download")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to grab release with status: {}", response.status());
        }

        anyhow::bail!(
            "Prowlarr requires downloading first. \
             Configure a local torrent client with automatic downloading, \
             then set the download folder in config.toml."
        )
    }
}
