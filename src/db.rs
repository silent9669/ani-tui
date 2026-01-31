use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone)]
pub struct WatchHistory {
    pub anime_id: String,
    pub provider: String,
    pub title: String,
    pub cover_url: String,
    pub episode_number: u32,
    pub episode_title: Option<String>,
    pub position_seconds: u64,
    pub total_seconds: u64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ImageCache {
    pub id: String,
    pub url: String,
    pub data: Vec<u8>,
    pub accessed_at: DateTime<Utc>,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let db_path = Self::default_db_path()?;
        
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open database at {:?}", db_path))?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.init_tables().await?;

        Ok(db)
    }

    async fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().await;

        // Watch history table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS watch_history (
                anime_id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                title TEXT NOT NULL,
                cover_url TEXT NOT NULL,
                episode_number INTEGER NOT NULL,
                episode_title TEXT,
                position_seconds INTEGER NOT NULL,
                total_seconds INTEGER NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // Image cache table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS image_cache (
                id TEXT PRIMARY KEY,
                url TEXT UNIQUE NOT NULL,
                data BLOB NOT NULL,
                accessed_at TEXT NOT NULL
            )",
            [],
        )?;

        // Metadata cache table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS metadata_cache (
                anilist_id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                rating INTEGER,
                cover_url TEXT,
                genres TEXT,
                episode_count INTEGER,
                cached_at TEXT NOT NULL
            )",
            [],
        )?;

        // Favorites table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS favorites (
                anime_id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                title TEXT NOT NULL,
                cover_url TEXT NOT NULL,
                added_at TEXT NOT NULL
            )",
            [],
        )?;

        // Indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_watch_history_updated 
             ON watch_history(updated_at DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_image_cache_accessed 
             ON image_cache(accessed_at)",
            [],
        )?;

        Ok(())
    }

    pub async fn save_watch_history(&self, history: &WatchHistory) -> Result<()> {
        let conn = self.conn.lock().await;
        
        conn.execute(
            "INSERT OR REPLACE INTO watch_history 
             (anime_id, provider, title, cover_url, episode_number, episode_title, 
              position_seconds, total_seconds, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                history.anime_id,
                history.provider,
                history.title,
                history.cover_url,
                history.episode_number,
                history.episode_title,
                history.position_seconds,
                history.total_seconds,
                history.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub async fn get_watch_history(&self, anime_id: &str) -> Result<Option<WatchHistory>> {
        let conn = self.conn.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT anime_id, provider, title, cover_url, episode_number, episode_title,
                    position_seconds, total_seconds, updated_at
             FROM watch_history WHERE anime_id = ?1"
        )?;

        let history = stmt.query_row([anime_id], |row| {
            Ok(WatchHistory {
                anime_id: row.get(0)?,
                provider: row.get(1)?,
                title: row.get(2)?,
                cover_url: row.get(3)?,
                episode_number: row.get(4)?,
                episode_title: row.get(5)?,
                position_seconds: row.get(6)?,
                total_seconds: row.get(7)?,
                updated_at: row.get::<_, String>(8)?.parse().unwrap_or_else(|_| Utc::now()),
            })
        }).ok();

        Ok(history)
    }

    pub async fn get_continue_watching(&self, limit: usize,
) -> Result<Vec<WatchHistory>> {
        let conn = self.conn.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT anime_id, provider, title, cover_url, episode_number, episode_title,
                    position_seconds, total_seconds, updated_at
             FROM watch_history 
             ORDER BY updated_at DESC
             LIMIT ?1"
        )?;

        let histories: Vec<WatchHistory> = stmt
            .query_map([limit], |row| {
                Ok(WatchHistory {
                    anime_id: row.get(0)?,
                    provider: row.get(1)?,
                    title: row.get(2)?,
                    cover_url: row.get(3)?,
                    episode_number: row.get(4)?,
                    episode_title: row.get(5)?,
                    position_seconds: row.get(6)?,
                    total_seconds: row.get(7)?,
                    updated_at: row.get::<_, String>(8)?.parse().unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<_, _>>()?;

        Ok(histories)
    }

    pub async fn cache_image(&self, id: &str, url: &str, data: &[u8],
    ) -> Result<()> {
        let conn = self.conn.lock().await;
        
        conn.execute(
            "INSERT OR REPLACE INTO image_cache (id, url, data, accessed_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                id,
                url,
                data,
                Utc::now().to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub async fn get_cached_image(&self, id: &str,
    ) -> Result<Option<ImageCache>> {
        let conn = self.conn.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, url, data, accessed_at FROM image_cache WHERE id = ?1"
        )?;

        let cache = stmt.query_row([id], |row| {
            Ok(ImageCache {
                id: row.get(0)?,
                url: row.get(1)?,
                data: row.get(2)?,
                accessed_at: row.get::<_, String>(3)?.parse().unwrap_or_else(|_| Utc::now()),
            })
        }).ok();

        // Update access time
        if cache.is_some() {
            conn.execute(
                "UPDATE image_cache SET accessed_at = ?1 WHERE id = ?2",
                params![Utc::now().to_rfc3339(), id],
            )?;
        }

        Ok(cache)
    }

    pub async fn cleanup_old_images(&self, max_size_mb: usize,
    ) -> Result<()> {
        let conn = self.conn.lock().await;
        
        // Calculate current cache size
        let size_mb: f64 = conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(data)), 0) / (1024.0 * 1024.0) FROM image_cache",
            [],
            |row| row.get(0),
        )?;

        if size_mb > max_size_mb as f64 {
            // Delete oldest entries until under limit
            let to_delete = ((size_mb - max_size_mb as f64) / 0.5) as i64;
            
            conn.execute(
                "DELETE FROM image_cache WHERE id IN (
                    SELECT id FROM image_cache ORDER BY accessed_at ASC LIMIT ?1
                )",
                [to_delete],
            )?;
        }

        Ok(())
    }

    pub async fn cache_metadata(
        &self,
        metadata: &crate::metadata::AniListMetadata,
    ) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT OR REPLACE INTO metadata_cache 
             (anilist_id, title, description, rating, cover_url, genres, episode_count, cached_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                metadata.anilist_id,
                metadata.title,
                metadata.description.as_ref().unwrap_or(&String::new()),
                metadata.rating.unwrap_or(0),
                metadata.cover_url.as_ref().unwrap_or(&String::new()),
                serde_json::to_string(&metadata.genres)?,
                metadata.episode_count.unwrap_or(0),
                metadata.cached_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub async fn get_cached_metadata(
        &self,
        anilist_id: i64,
    ) -> Result<Option<crate::metadata::AniListMetadata>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT anilist_id, title, description, rating, cover_url, genres, episode_count, cached_at
             FROM metadata_cache WHERE anilist_id = ?1"
        )?;

        let metadata = stmt.query_row([anilist_id], |row| {
            let genres_str: String = row.get(5)?;
            let genres: Vec<String> = serde_json::from_str(&genres_str).unwrap_or_default();

            Ok(crate::metadata::AniListMetadata {
                anilist_id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                rating: row.get(3)?,
                cover_url: row.get(4)?,
                genres,
                episode_count: row.get(6)?,
                cached_at: row.get::<_, String>(7)?.parse().unwrap_or_else(|_| Utc::now()),
            })
        }).ok();

        Ok(metadata)
    }

    pub async fn save_favorite(
        &self,
        anime_id: &str,
        provider: &str,
        title: &str,
        cover_url: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT OR REPLACE INTO favorites 
             (anime_id, provider, title, cover_url, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                anime_id,
                provider,
                title,
                cover_url,
                Utc::now().to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub async fn remove_favorite(&self, anime_id: &str) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM favorites WHERE anime_id = ?1",
            params![anime_id],
        )?;

        Ok(())
    }

    pub async fn is_favorite(&self, anime_id: &str) -> Result<bool> {
        let conn = self.conn.lock().await;

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM favorites WHERE anime_id = ?1",
            params![anime_id],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    pub async fn get_favorites(
        &self,
        limit: usize,
    ) -> Result<Vec<(String, String, String, String)>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT anime_id, provider, title, cover_url
             FROM favorites 
             ORDER BY added_at DESC
             LIMIT ?1"
        )?;

        let favorites: Vec<(String, String, String, String)> = stmt
            .query_map([limit], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                ))
            })?
            .collect::<Result<_, _>>()?;

        Ok(favorites)
    }

    fn default_db_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "ani-tui", "ani-tui")
            .context("Failed to determine data directory")?;
        Ok(proj_dirs.data_dir().join("history.db"))
    }
}
