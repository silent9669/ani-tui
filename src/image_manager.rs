use crate::db::Database;
use anyhow::Result;
use image::DynamicImage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ImageManager {
    db: Arc<Database>,
    cache: Arc<RwLock<HashMap<String, DynamicImage>>>,
    client: reqwest::Client,
}

impl ImageManager {
    pub fn new(db: Arc<Database>) -> Self {
        let client = reqwest::Client::new();
        Self {
            db,
            cache: Arc::new(RwLock::new(HashMap::new())),
            client,
        }
    }

    pub async fn get_image(&self,
        id: &str,
        url: &str,
    ) -> Result<Option<DynamicImage>> {
        // Check memory cache first
        {
            let cache = self.cache.read().await;
            if let Some(img) = cache.get(id) {
                return Ok(Some(img.clone()));
            }
        }

        // Check database cache
        if let Some(cached) = self.db.get_cached_image(id).await? {
            if let Ok(img) = image::load_from_memory(&cached.data) {
                // Store in memory cache
                let mut cache = self.cache.write().await;
                cache.insert(id.to_string(), img.clone());
                return Ok(Some(img));
            }
        }

        // Download image
        match self.download_image(url).await {
            Ok(img) => {
                // Save to database
                if let Ok(bytes) = self.image_to_bytes(&img) {
                    let _ = self.db.cache_image(id, url, &bytes).await;
                }

                // Save to memory cache
                let mut cache = self.cache.write().await;
                cache.insert(id.to_string(), img.clone());

                Ok(Some(img))
            }
            Err(_) => {
                Ok(None)
            }
        }
    }

    async fn download_image(&self,
        url: &str,
    ) -> Result<DynamicImage> {
        let response = self.client.get(url).send().await?;
        let bytes = response.bytes().await?;
        let img = image::load_from_memory(&bytes)?;
        Ok(img)
    }

    fn image_to_bytes(&self,
        img: &DynamicImage,
    ) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageFormat::Jpeg)?;
        Ok(buffer)
    }

    pub async fn clear_memory_cache(&self,
    ) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}
