use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub player: PlayerConfig,

    #[serde(default)]
    pub ui: UiConfig,

    #[serde(default)]
    pub sources: SourcesConfig,

    #[serde(default)]
    pub prowlarr: Option<ProwlarrConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerConfig {
    #[serde(default = "default_player")]
    pub default: String,

    #[serde(default)]
    pub quality: Option<String>,

    #[serde(default)]
    pub no_subs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_true")]
    pub image_preview: bool,

    #[serde(default = "default_theme")]
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesConfig {
    #[serde(default = "default_true")]
    pub allanime: bool,

    #[serde(default)]
    pub gogoanime: bool,

    #[serde(default = "default_true")]
    pub vietnamese: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProwlarrConfig {
    pub url: String,
    pub api_key: String,
    #[serde(default)]
    pub indexer: u32,
}

fn default_player() -> String {
    "mpv".to_string()
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            default: default_player(),
            quality: None,
            no_subs: false,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            image_preview: true,
            theme: default_theme(),
        }
    }
}

impl Default for SourcesConfig {
    fn default() -> Self {
        Self {
            allanime: true,
            gogoanime: false,
            vietnamese: true,
        }
    }
}

impl Config {
    pub async fn load(path: Option<&str>) -> Result<Self> {
        let config_path = if let Some(p) = path {
            PathBuf::from(p)
        } else {
            Self::default_config_path()?
        };

        if !config_path.exists() {
            tracing::info!(
                "Config file not found, creating default at {:?}",
                config_path
            );
            let config = Config::default();
            config.save(&config_path).await?;
            return Ok(config);
        }

        let content = tokio::fs::read_to_string(&config_path)
            .await
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {:?}", config_path))?;

        Ok(config)
    }

    pub async fn save(&self, path: &PathBuf) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = toml::to_string_pretty(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    pub fn default_config_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "ani-tui", "ani-tui")
            .context("Failed to determine config directory")?;
        Ok(proj_dirs.config_dir().join("config.toml"))
    }

    pub fn validate(&self) -> Result<()> {
        // Config validation - all providers now work without external dependencies
        Ok(())
    }
}
