pub mod allanime;
pub mod kkphim;
pub mod ophim;

use crate::config::Config;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anime {
    pub id: String,
    pub provider: String,
    pub title: String,
    pub cover_url: String,
    pub language: Language,
    pub total_episodes: Option<u32>,
    pub synopsis: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub number: u32,
    pub title: Option<String>,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub video_url: String,
    pub subtitles: Vec<Subtitle>,
    pub qualities: Vec<String>,
    pub headers: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtitle {
    pub language: String,
    pub url: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Language {
    English,
    Vietnamese,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::English => write!(f, "EN"),
            Language::Vietnamese => write!(f, "VN"),
        }
    }
}

#[async_trait]
pub trait AnimeProvider: Send + Sync {
    fn name(&self) -> &str;
    fn language(&self) -> Language;
    fn supported_languages(&self) -> Vec<String>;

    async fn search(&self, query: &str) -> Result<Vec<Anime>>;
    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>>;
    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo>;
}

pub struct ProviderRegistry {
    providers: Vec<Arc<dyn AnimeProvider>>,
}

impl ProviderRegistry {
    pub fn new(config: &Config) -> Self {
        let mut providers: Vec<Arc<dyn AnimeProvider>> = Vec::new();

        // --- English Sources ---
        // 1. AllAnime (Anime & Films)
        if config.sources.allanime {
            providers.push(Arc::new(allanime::AllAnimeProvider::new()));
        }

        // --- Vietnamese Sources ---
        // 2. KKPhim
        if config.sources.kkphim {
            providers.push(Arc::new(kkphim::KkphimProvider::new()));
        }

        // 3. OPhim
        if config.sources.ophim {
            providers.push(Arc::new(ophim::OphimProvider::new()));
        }

        Self { providers }
    }

    pub async fn search_all(&self, query: &str) -> Result<Vec<Anime>> {
        let mut all_results = Vec::new();

        for provider in &self.providers {
            if let Ok(mut results) = provider.search(query).await {
                all_results.append(&mut results);
            }
        }

        Ok(all_results)
    }

    pub async fn search_filtered(&self, query: &str, languages: &[Language]) -> Result<Vec<Anime>> {
        let mut all_results = Vec::new();

        for provider in &self.providers {
            // Only search providers that match the selected languages
            if languages.contains(&provider.language()) {
                if let Ok(mut results) = provider.search(query).await {
                    all_results.append(&mut results);
                }
            }
        }

        Ok(all_results)
    }

    pub fn get_provider(&self, name: &str) -> Option<&Arc<dyn AnimeProvider>> {
        self.providers.iter().find(|p| p.name() == name)
    }

    pub fn list_providers(&self) -> &[Arc<dyn AnimeProvider>] {
        &self.providers
    }
}
