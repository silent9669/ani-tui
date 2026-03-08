use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

const MAX_CONCURRENT_DOWNLOADS: usize = 10;
const MAX_MEMORY_CACHE: usize = 50;

#[derive(Debug, Clone)]
pub struct CachedImage {
    pub id: String,
    pub url: String,
    pub data: Vec<u8>,
    pub accessed_at: std::time::Instant,
}

pub struct ImagePipeline {
    #[allow(dead_code)]
    client: reqwest::Client,
    memory_cache: Arc<RwLock<HashMap<String, CachedImage>>>,
    db: Arc<crate::db::Database>,
    download_tx: mpsc::Sender<DownloadRequest>,
}

#[derive(Debug)]
struct DownloadRequest {
    id: String,
    url: String,
    result_tx: mpsc::Sender<Result<Vec<u8>>>,
}

pub struct AsciiRenderer;

impl AsciiRenderer {
    pub fn new() -> Self {
        Self
    }

    pub fn is_available() -> bool {
        // image_ascii is always available since it's a Rust crate
        true
    }

    pub fn render(&self, image_data: &[u8], _width: u32, _height: u32) -> Result<String> {
        // Validate image data
        if image_data.is_empty() {
            anyhow::bail!("Image data is empty");
        }

        // Check for valid image magic bytes
        let is_valid_image = image_data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) // PNG
            || image_data.starts_with(&[0xFF, 0xD8, 0xFF]) // JPEG
            || image_data.starts_with(&[0x52, 0x49, 0x46, 0x46]) // WEBP
            || image_data.starts_with(&[0x47, 0x49, 0x46, 0x38, 0x37, 0x61]) // GIF87a
            || image_data.starts_with(&[0x47, 0x49, 0x46, 0x38, 0x39, 0x61]) // GIF89a
            || image_data.starts_with(&[0x42, 0x4D]) // BMP
            || image_data.starts_with(&[0x49, 0x49, 0x2A, 0x00]) // TIFF little-endian
            || image_data.starts_with(&[0x4D, 0x4D, 0x00, 0x2A]); // TIFF big-endian

        if !is_valid_image {
            tracing::warn!("Invalid image format, {} bytes", image_data.len());
            anyhow::bail!("Invalid image format");
        }

        // Use image_ascii for in-memory image rendering
        // First, parse the image data using the image crate
        let img = image::load_from_memory(image_data).context("Failed to parse image data")?;

        // Create ASCII text generator
        let generator = image_ascii::TextGenerator::new(&img);

        // Generate ASCII art
        let ascii = generator.generate();

        Ok(ascii)
    }

    pub fn render_placeholder(_width: u32, height: u32) -> String {
        let lines = vec!["[No image]".to_string(); (height as usize).min(10)];
        lines.join("\n")
    }
}

impl ImagePipeline {
    pub fn new(db: Arc<crate::db::Database>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        let memory_cache = Arc::new(RwLock::new(HashMap::with_capacity(MAX_MEMORY_CACHE)));
        let (download_tx, mut download_rx) = mpsc::channel::<DownloadRequest>(100);

        let client_clone = client.clone();
        let cache_clone = memory_cache.clone();
        let db_clone = db.clone();

        // Spawn background download worker
        tokio::spawn(async move {
            let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_DOWNLOADS));

            while let Some(request) = download_rx.recv().await {
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let client = client_clone.clone();
                let cache = cache_clone.clone();
                let db = db_clone.clone();
                let result_tx = request.result_tx.clone();

                tokio::spawn(async move {
                    let _permit = permit;
                    let result =
                        Self::download_and_cache(client, cache, db, &request.id, &request.url)
                            .await;

                    let _ = result_tx.send(result).await;
                });
            }
        });

        Self {
            client,
            memory_cache,
            db,
            download_tx,
        }
    }

    async fn download_and_cache(
        client: reqwest::Client,
        cache: Arc<RwLock<HashMap<String, CachedImage>>>,
        db: Arc<crate::db::Database>,
        id: &str,
        url: &str,
    ) -> Result<Vec<u8>> {
        // Check disk cache first
        if let Some(cached) = db.get_cached_image(id).await? {
            tracing::debug!("Image {} loaded from disk cache", id);

            // Add to memory cache
            let image = CachedImage {
                id: id.to_string(),
                url: url.to_string(),
                data: cached.data.clone(),
                accessed_at: std::time::Instant::now(),
            };

            let mut cache_guard = cache.write().await;
            Self::insert_into_memory_cache(&mut cache_guard, id.to_string(), image);
            drop(cache_guard);

            return Ok(cached.data);
        }

        // Download from URL
        tracing::debug!("Downloading image {} from {}", id, url);
        let response = client.get(url).send().await?;
        let data = response.bytes().await?.to_vec();

        // Save to disk cache
        let _ = db.cache_image(id, url, &data).await;

        // Add to memory cache
        let image = CachedImage {
            id: id.to_string(),
            url: url.to_string(),
            data: data.clone(),
            accessed_at: std::time::Instant::now(),
        };

        let mut cache_guard = cache.write().await;
        Self::insert_into_memory_cache(&mut cache_guard, id.to_string(), image);

        Ok(data)
    }

    fn insert_into_memory_cache(
        cache: &mut HashMap<String, CachedImage>,
        id: String,
        image: CachedImage,
    ) {
        // LRU eviction
        if cache.len() >= MAX_MEMORY_CACHE {
            // Find oldest accessed entry and remove it
            let oldest_id = cache
                .iter()
                .min_by_key(|(_, v)| v.accessed_at)
                .map(|(k, _)| k.clone());

            if let Some(oldest_id) = oldest_id {
                cache.remove(&oldest_id);
            }
        }

        cache.insert(id, image);
    }

    pub async fn get_image(&self, id: &str) -> Option<CachedImage> {
        // Check memory cache first
        let cache_guard = self.memory_cache.read().await;
        if let Some(image) = cache_guard.get(id) {
            return Some(image.clone());
        }
        drop(cache_guard);

        None
    }

    pub async fn request_download(&self, id: String, url: String) -> Result<Vec<u8>> {
        // Check memory cache first
        let cache_guard = self.memory_cache.read().await;
        if let Some(image) = cache_guard.get(&id) {
            return Ok(image.data.clone());
        }
        drop(cache_guard);

        // Check disk cache
        if let Some(cached) = self.db.get_cached_image(&id).await? {
            // Add to memory cache
            let image = CachedImage {
                id: id.clone(),
                url: url.clone(),
                data: cached.data.clone(),
                accessed_at: std::time::Instant::now(),
            };

            let mut cache_guard = self.memory_cache.write().await;
            Self::insert_into_memory_cache(&mut cache_guard, id.clone(), image);
            drop(cache_guard);

            return Ok(cached.data);
        }

        // Send download request
        let (result_tx, mut result_rx) = mpsc::channel(1);
        let request = DownloadRequest { id, url, result_tx };

        let _ = self.download_tx.send(request).await;

        // Wait for result
        result_rx
            .recv()
            .await
            .context("Download worker closed")?
            .context("Download failed")
    }

    pub async fn preload_images(&self, images: Vec<(String, String)>) {
        for (id, url) in images {
            // Check if already in cache
            let cache_guard = self.memory_cache.read().await;
            if cache_guard.contains_key(&id) {
                continue;
            }
            drop(cache_guard);

            // Send download request without waiting
            let (result_tx, _result_rx) = mpsc::channel(1);
            let request = DownloadRequest { id, url, result_tx };
            let _ = self.download_tx.send(request).await;
        }
    }
}

impl Default for AsciiRenderer {
    fn default() -> Self {
        Self::new()
    }
}
