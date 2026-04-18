use super::{Anime, AnimeProvider, Episode, Language, StreamInfo};
use aes::cipher::{KeyIvInit, StreamCipher};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::header::{self, HeaderMap};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;

const ALLANIME_API: &str = "https://api.allanime.day/api";
const ALLANIME_REFERRER: &str = "https://allmanga.to";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub struct AllAnimeProvider {
    client: reqwest::Client,
}

impl Default for AllAnimeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AllAnimeProvider {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(
            header::REFERER,
            header::HeaderValue::from_static(ALLANIME_REFERRER),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    pub fn decrypt_tobeparsed(encrypted: &str) -> Result<String> {
        let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encrypted)
            .context("Failed to decode base64 tobeparsed")?;

        // Format: IV (12 bytes) + Ciphertext + Signature (16 bytes)
        if decoded.len() < 28 {
            anyhow::bail!("Encrypted data too short");
        }

        // Key = Sha256("SimtVuagFbGR2K7P")
        let secret = "SimtVuagFbGR2K7P";
        let mut hasher = Sha256::new();
        hasher.update(secret);
        let key = hasher.finalize();

        // IV = first 12 bytes + counter "00000002"
        let iv_bytes = &decoded[0..12];
        let mut iv = [0u8; 16];
        iv[0..12].copy_from_slice(iv_bytes);
        iv[15] = 2; // Counter starts at 2 as per ani-cli decode_tobeparsed logic

        // Ciphertext is after IV and before the last 16 bytes (signature)
        let ciphertext_end = decoded.len() - 16;
        let ciphertext = &decoded[12..ciphertext_end];
        let mut data = ciphertext.to_vec();

        type Aes256Ctr = ctr::Ctr128BE<aes::Aes256>;
        let mut cipher = Aes256Ctr::new(&key, &iv.into());
        cipher.apply_keystream(&mut data);

        let decrypted = String::from_utf8(data).context("Failed to parse decrypted UTF-8")?;
        Ok(decrypted)
    }

    async fn graphql_query(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let response: serde_json::Value = self
            .client
            .post(ALLANIME_API)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "variables": variables,
                "query": query
            }))
            .send()
            .await
            .context("GraphQL request failed")?
            .json()
            .await
            .context("Failed to parse GraphQL response")?;

        // Check if data is wrapped in tobeparsed
        if let Some(data) = response.get("data") {
            if let Some(tobeparsed) = data["tobeparsed"].as_str() {
                let decrypted = Self::decrypt_tobeparsed(tobeparsed)?;
                return serde_json::from_str(&decrypted).context("Failed to parse decrypted JSON");
            }
        }

        Ok(response)
    }

    pub fn decode_provider_id(encoded: &str) -> String {
        let encoded = encoded.trim_start_matches("--");
        let mut result = String::new();
        let chars: Vec<char> = encoded.chars().collect();

        for chunk in chars.chunks(2) {
            if chunk.len() == 2 {
                let hex = format!("{}{}", chunk[0], chunk[1]);
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    let ch = match byte {
                        0x79 => 'A',
                        0x7a => 'B',
                        0x7b => 'C',
                        0x7c => 'D',
                        0x7d => 'E',
                        0x7e => 'F',
                        0x7f => 'G',
                        0x70 => 'H',
                        0x71 => 'I',
                        0x72 => 'J',
                        0x73 => 'K',
                        0x74 => 'L',
                        0x75 => 'M',
                        0x76 => 'N',
                        0x77 => 'O',
                        0x68 => 'P',
                        0x69 => 'Q',
                        0x6a => 'R',
                        0x6b => 'S',
                        0x6c => 'T',
                        0x6d => 'U',
                        0x6e => 'V',
                        0x6f => 'W',
                        0x60 => 'X',
                        0x61 => 'Y',
                        0x62 => 'Z',
                        0x59 => 'a',
                        0x5a => 'b',
                        0x5b => 'c',
                        0x5c => 'd',
                        0x5d => 'e',
                        0x5e => 'f',
                        0x5f => 'g',
                        0x50 => 'h',
                        0x51 => 'i',
                        0x52 => 'j',
                        0x53 => 'k',
                        0x54 => 'l',
                        0x55 => 'm',
                        0x56 => 'n',
                        0x57 => 'o',
                        0x48 => 'p',
                        0x49 => 'q',
                        0x4a => 'r',
                        0x4b => 's',
                        0x4c => 't',
                        0x4d => 'u',
                        0x4e => 'v',
                        0x4f => 'w',
                        0x40 => 'x',
                        0x41 => 'y',
                        0x42 => 'z',
                        0x08 => '0',
                        0x09 => '1',
                        0x0a => '2',
                        0x0b => '3',
                        0x0c => '4',
                        0x0d => '5',
                        0x0e => '6',
                        0x0f => '7',
                        0x00 => '8',
                        0x01 => '9',
                        0x15 => '-',
                        0x16 => '.',
                        0x67 => '_',
                        0x46 => '~',
                        0x02 => ':',
                        0x17 => '/',
                        0x07 => '?',
                        0x1b => '#',
                        0x63 => '[',
                        0x65 => ']',
                        0x78 => '@',
                        0x19 => '!',
                        0x1c => '$',
                        0x1e => '&',
                        0x10 => '(',
                        0x11 => ')',
                        0x12 => '*',
                        0x13 => '+',
                        0x14 => ',',
                        0x03 => ';',
                        0x05 => '=',
                        0x1d => '%',
                        b => {
                            if b.is_ascii_graphic() || b == b'/' || b == b'.' {
                                b as char
                            } else {
                                continue;
                            }
                        }
                    };
                    result.push(ch);
                }
            }
        }

        result
            .replace("/clock", "/clock.json")
            .replace("/clock.json.json", "/clock.json")
    }
}

#[async_trait]
impl AnimeProvider for AllAnimeProvider {
    fn name(&self) -> &str {
        "AllAnime"
    }

    fn language(&self) -> Language {
        Language::English
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["🇺🇸".to_string()]
    }

    async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        let search_gql = r#"query($search: SearchInput $limit: Int $page: Int $translationType: VaildTranslationTypeEnumType $countryOrigin: VaildCountryOriginEnumType) { shows(search: $search limit: $limit page: $page translationType: $translationType countryOrigin: $countryOrigin) { edges { _id name availableEpisodes thumbnail __typename } }}"#;

        let variables = serde_json::json!({
            "search": {
                "allowAdult": false,
                "allowUnknown": false,
                "query": query
            },
            "limit": 40,
            "page": 1,
            "translationType": "sub",
            "countryOrigin": "ALL"
        });

        let response = self.graphql_query(search_gql, variables).await?;
        let mut results = Vec::new();

        let shows = if let Some(data) = response.get("data") {
            &data["shows"]
        } else {
            &response["shows"]
        };

        if let Some(edges) = shows["edges"].as_array() {
            for edge in edges {
                let id = edge["_id"].as_str().unwrap_or_default().to_string();
                let name = edge["name"].as_str().unwrap_or_default().to_string();
                let thumbnail = edge["thumbnail"].as_str().unwrap_or_default().to_string();
                let episodes = edge["availableEpisodes"]["sub"].as_u64().map(|n| n as u32);

                if !id.is_empty() && !name.is_empty() {
                    results.push(Anime {
                        id,
                        provider: "AllAnime".to_string(),
                        title: name,
                        cover_url: thumbnail,
                        language: Language::English,
                        total_episodes: episodes,
                        synopsis: None,
                    });
                }
            }
        }

        Ok(results)
    }

    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        let episodes_gql =
            r#"query($showId: String!) { show(_id: $showId) { _id availableEpisodesDetail }}"#;

        let variables = serde_json::json!({
            "showId": anime_id
        });

        let response = self.graphql_query(episodes_gql, variables).await?;
        let mut episodes = Vec::new();

        let show = if let Some(data) = response.get("data") {
            &data["show"]
        } else {
            &response["show"]
        };

        if let Some(episode_list) = show["availableEpisodesDetail"]["sub"].as_array() {
            for (idx, ep) in episode_list.iter().enumerate() {
                if let Some(ep_num) = ep.as_str() {
                    episodes.push(Episode {
                        number: ep_num.parse().unwrap_or((idx + 1) as u32),
                        title: None,
                        thumbnail: None,
                    });
                }
            }
        }

        episodes.sort_by(|a, b| a.number.cmp(&b.number));
        Ok(episodes)
    }

    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo> {
        let parts: Vec<&str> = episode_id.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid episode_id format. Expected 'anime_id:episode_number'");
        }

        let anime_id = parts[0];
        let episode_number = parts[1];

        let embed_gql = r#"query($showId: String!, $translationType: VaildTranslationTypeEnumType!, $episodeString: String!) { episode(showId: $showId translationType: $translationType episodeString: $episodeString) { episodeString sourceUrls }}"#;

        let variables = serde_json::json!({
            "showId": anime_id,
            "translationType": "sub",
            "episodeString": episode_number
        });

        let response = self.graphql_query(embed_gql, variables).await?;
        let mut stream_url = String::new();
        let subtitles = Vec::new();
        let mut qualities = Vec::new();
        let mut headers = HashMap::new();

        let episode = if let Some(data) = response.get("data") {
            &data["episode"]
        } else {
            &response["episode"]
        };

        if let Some(source_urls) = episode["sourceUrls"].as_array() {
            // Priority order: direct URLs first (Ok.ru, mp4upload, filemoon), then others
            let priority_sources = [
                "Ok", "Mp4", "Fm-Hls", "Yt-mp4", "S-mp4", "Luf-Mp4", "Sup", "Uni",
            ];

            for priority_name in &priority_sources {
                if let Some(source) = source_urls
                    .iter()
                    .find(|s| s["sourceName"].as_str() == Some(*priority_name))
                {
                    if let Some(source_url_encoded) = source["sourceUrl"].as_str() {
                        let decoded = Self::decode_provider_id(source_url_encoded);

                        // For direct HTTP URLs (ok.ru, mp4upload, filemoon), use them directly
                        if decoded.starts_with("http") {
                            stream_url = decoded;
                            qualities.push("auto".to_string());
                            break;
                        }

                        // For relative URLs (clock.json), skip them for now as they require special handling
                        if decoded.starts_with("/") {
                            continue;
                        }
                    }
                }
            }
        }

        if stream_url.is_empty() {
            anyhow::bail!(
                "No working stream URL found. This might be a temporary issue with AllAnime."
            );
        }

        headers.insert("Referer".to_string(), ALLANIME_REFERRER.to_string());

        Ok(StreamInfo {
            video_url: stream_url,
            subtitles,
            qualities,
            headers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_provider_id() {
        let encoded = "79677a7a78";
        let decoded = AllAnimeProvider::decode_provider_id(encoded);
        assert!(!decoded.is_empty());
    }
}
