use super::{parse_episode_number, Anime, AnimeProvider, Episode, Language, StreamInfo, Subtitle};
use aes::cipher::{KeyIvInit, StreamCipher};
use aes_gcm::{
    aead::{Aead, KeyInit as AeadKeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::Engine as _;
use hmac::Mac;
use regex::Regex;
use reqwest::header::{self, HeaderMap};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

const ALLANIME_API: &str = "https://api.mkissa.net/api";
const ALLANIME_CRYPTO_BOOTSTRAP: &str = "https://api.mkissa.net/client-crypto/v1/bootstrap";
const ALLANIME_BUILD_ID: &str = "110";
const ALLANIME_KEY_GROUP: &str = "mkissa";
const ALLANIME_HOST: &str = "mkissa.to";
const ALLANIME_LANE: &str = "k7";
const ALLANIME_QUERY_HASH_SEARCH: &str =
    "b1c9564668f81293003be988b1f6594d8ce3951f2a16bb4c64d896471d678077";
const ALLANIME_QUERY_HASH_DETAIL: &str =
    "e0a219831b88c540498c7c4cfde2c174a58296db92d98ce64d6b2a66326c6fbf";
const ALLANIME_QUERY_HASH_EPISODE: &str =
    "3dd139fb89a54cddb2b167a23e274c49a2ec9ab036ab64d2b39f92c0e9dd888e";
const MP4UPLOAD_REFERRER: &str = "https://www.mp4upload.com";
const ALLANIME_REFERRER: &str = "https://youtu-chan.com";
const ALLANIME_ORIGIN: &str = "https://mkissa.to";
const ALLANIME_BASE: &str = "https://mkissa.to";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
struct AllAnimeCryptoState {
    expires_at_ms: u64,
    epoch: u64,
    key: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StreamCandidate {
    url: String,
    quality: Option<u32>,
    label: String,
    headers: HashMap<String, String>,
}

impl StreamCandidate {
    fn new(url: String, label: impl Into<String>) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Referer".to_string(), ALLANIME_REFERRER.to_string());
        headers.insert("User-Agent".to_string(), USER_AGENT.to_string());

        Self {
            url,
            quality: None,
            label: label.into(),
            headers,
        }
    }

    fn with_quality(mut self, quality: Option<u32>) -> Self {
        self.quality = quality;
        self
    }

    fn with_referrer(mut self, referrer: impl Into<String>) -> Self {
        self.headers.insert("Referer".to_string(), referrer.into());
        self
    }
}

pub struct AllAnimeProvider {
    client: reqwest::Client,
    insecure_client: reqwest::Client,
    crypto: RwLock<Option<AllAnimeCryptoState>>,
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
            header::HeaderValue::from_static(USER_AGENT),
        );
        headers.insert(
            header::REFERER,
            header::HeaderValue::from_static(ALLANIME_ORIGIN),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers.clone())
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("Failed to create HTTP client");

        let insecure_client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(REQUEST_TIMEOUT)
            .danger_accept_invalid_certs(true)
            .build()
            .expect("Failed to create insecure HTTP client");

        Self {
            client,
            insecure_client,
            crypto: RwLock::new(None),
        }
    }

    fn unix_time_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn build_mask(build_id: &str) -> [u8; 32] {
        // Mirrors the runtime `b_` derivation in the Mkissa crypto bundle:
        // four base64 fragments are XOR-mixed with a build-id key stream and
        // a per-part/per-byte constant, producing a 32-byte mask that is
        // epoch-independent.
        const XD: [&str; 4] = [
            "bYbiqMWaGN0=",
            "1wgQ45B/tts=",
            "0dDkHy33wIU=",
            "vHKzMJg/BJ4=",
        ];
        let id_bytes = build_id.as_bytes();
        let mut stream = [0u8; 32];
        for (index, byte) in stream.iter_mut().enumerate() {
            *byte = id_bytes[index % id_bytes.len()] ^ ((index * 17 + 31) & 255) as u8;
        }

        let mut mask = [0u8; 32];
        for (part, fragment) in XD.iter().enumerate() {
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(fragment)
                .expect("AllAnime mask fragment is valid base64");
            let offset = part * 8;
            for (index, data) in decoded.iter().enumerate() {
                mask[offset + index] =
                    data ^ stream[offset + index] ^ (part as u8 * 41 + index as u8 * 7);
            }
        }
        mask
    }

    fn current_epoch() -> u64 {
        let now = Self::unix_time_ms();
        let epoch = now / 604_800_000;
        if epoch > 0 && now - epoch * 604_800_000 < 86_400_000 {
            epoch - 1
        } else {
            epoch
        }
    }

    fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
        type HmacSha256 = hmac::Hmac<Sha256>;
        let mut mac =
            <HmacSha256 as Mac>::new_from_slice(key).expect("HMAC accepts any key length");
        mac.update(message);
        mac.finalize().into_bytes().to_vec()
    }

    fn boot_token(
        mask: &[u8; 32],
        build_id: &str,
        key_group: &str,
        host: &str,
        epoch: u64,
        lane: &str,
    ) -> String {
        let digest = Self::hmac_sha256(mask, format!("aa-boot:{build_id}").as_bytes());
        let message = if lane.is_empty() {
            format!("{build_id}:{key_group}:{host}:{epoch}")
        } else {
            format!("{build_id}:{key_group}:{host}:{epoch}:{lane}")
        };
        let token = Self::hmac_sha256(&digest, message.as_bytes());
        token
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    }

    async fn fetch_live_crypto(&self) -> Result<AllAnimeCryptoState> {
        let mask = Self::build_mask(ALLANIME_BUILD_ID);
        let epoch = Self::current_epoch();
        let token = Self::boot_token(
            &mask,
            ALLANIME_BUILD_ID,
            ALLANIME_KEY_GROUP,
            ALLANIME_HOST,
            epoch,
            ALLANIME_LANE,
        );
        let bootstrap_url = format!(
            "{ALLANIME_CRYPTO_BOOTSTRAP}?buildId={}&k={}",
            ALLANIME_BUILD_ID, ALLANIME_LANE
        );
        let response: serde_json::Value = self
            .client
            .get(&bootstrap_url)
            .header("x-aa-boot", &token)
            .header("x-build-id", ALLANIME_BUILD_ID)
            .send()
            .await
            .context("Failed to fetch AllAnime crypto bootstrap")?
            .error_for_status()
            .context("AllAnime crypto bootstrap was rejected")?
            .json()
            .await
            .context("Failed to parse AllAnime crypto bootstrap")?;
        let part_b = response["partB"]
            .as_str()
            .context("AllAnime crypto bootstrap secret was missing")?;
        let epoch = response["epoch"]
            .as_u64()
            .context("AllAnime crypto bootstrap epoch was missing")?;
        let switch_at = response["switchAt"].as_u64().unwrap_or_default();
        let grace_ms = response["graceMs"].as_u64().unwrap_or_default();

        let part_b = base64::engine::general_purpose::STANDARD
            .decode(part_b)
            .context("Failed to decode AllAnime crypto secret")?;
        anyhow::ensure!(
            part_b.len() == mask.len(),
            "AllAnime crypto secret has an invalid length"
        );
        let mut key = [0u8; 32];
        for (index, byte) in key.iter_mut().enumerate() {
            *byte = part_b[index] ^ mask[index];
        }

        Ok(AllAnimeCryptoState {
            expires_at_ms: (switch_at + grace_ms).max(Self::unix_time_ms() + 60 * 60 * 1000),
            epoch,
            key,
        })
    }

    async fn current_crypto(&self) -> Result<AllAnimeCryptoState> {
        let now = Self::unix_time_ms();
        if let Some(cached) = self
            .crypto
            .read()
            .await
            .as_ref()
            .filter(|cached| cached.expires_at_ms > now)
            .cloned()
        {
            return Ok(cached);
        }

        let crypto = self
            .fetch_live_crypto()
            .await
            .context("AllAnime runtime crypto is unavailable")?;
        tracing::debug!(epoch = crypto.epoch, "AllAnime runtime crypto refreshed");
        *self.crypto.write().await = Some(crypto.clone());
        Ok(crypto)
    }

    async fn build_source_request_crypto(&self) -> Result<(AllAnimeCryptoState, String)> {
        let crypto = self.current_crypto().await?;
        let timestamp = Self::unix_time_ms() / 300_000 * 300_000;
        let iv_digest = Sha256::digest(
            format!(
                "{}:{}:{}:{}:{}",
                crypto.epoch,
                ALLANIME_BUILD_ID,
                ALLANIME_QUERY_HASH_EPISODE,
                timestamp,
                ALLANIME_LANE
            )
            .as_bytes(),
        );
        let mut iv = [0u8; 12];
        iv.copy_from_slice(&iv_digest[..12]);
        let payload = serde_json::to_vec(&serde_json::json!({
            "v": 1,
            "ts": timestamp,
            "epoch": crypto.epoch,
            "buildId": ALLANIME_BUILD_ID,
            "qh": ALLANIME_QUERY_HASH_EPISODE,
            "k": ALLANIME_LANE,
        }))?;
        let cipher = Aes256Gcm::new_from_slice(&crypto.key)
            .map_err(|_| anyhow::anyhow!("AllAnime request crypto key was invalid"))?;
        let encrypted = cipher
            .encrypt(Nonce::from_slice(&iv), payload.as_ref())
            .map_err(|_| anyhow::anyhow!("Failed to encrypt AllAnime request token"))?;
        let mut token = Vec::with_capacity(1 + iv.len() + encrypted.len());
        token.push(1);
        token.extend_from_slice(&iv);
        token.extend_from_slice(&encrypted);
        Ok((
            crypto,
            base64::engine::general_purpose::STANDARD.encode(token),
        ))
    }

    fn decrypt_source_payload(encrypted: &str, runtime_key: &[u8; 32]) -> Result<String> {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encrypted)
            .context("Failed to decode encrypted AllAnime source response")?;
        anyhow::ensure!(
            decoded.len() >= 29,
            "Encrypted AllAnime source response was too short"
        );
        let nonce = Nonce::from_slice(&decoded[1..13]);
        let cipher = Aes256Gcm::new_from_slice(runtime_key)
            .map_err(|_| anyhow::anyhow!("AllAnime response crypto key was invalid"))?;
        let plain = cipher
            .decrypt(nonce, &decoded[13..])
            .map_err(|_| anyhow::anyhow!("Failed to decrypt AllAnime source response"))?;
        String::from_utf8(plain).context("AllAnime source response was not valid UTF-8")
    }

    async fn graphql_query(
        &self,
        query_hash: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let extensions = serde_json::json!({
            "persistedQuery": {
                "version": 1,
                "sha256Hash": query_hash
            }
        });
        let api_url = format!(
            "{}?variables={}&extensions={}",
            ALLANIME_API,
            url::form_urlencoded::byte_serialize(variables.to_string().as_bytes())
                .collect::<String>(),
            url::form_urlencoded::byte_serialize(extensions.to_string().as_bytes())
                .collect::<String>()
        );
        self.client
            .get(&api_url)
            .header("x-build-id", ALLANIME_BUILD_ID)
            .send()
            .await
            .context("GraphQL request failed")?
            .error_for_status()
            .context("GraphQL request returned an error")?
            .json()
            .await
            .context("Failed to parse GraphQL response")
    }

    pub fn decode_provider_id(encoded: &str) -> String {
        let encoded = encoded.trim_start_matches("--");
        let mut result = String::new();
        let chars: Vec<char> = encoded.chars().collect();

        let mut decoded_xor = String::new();
        for chunk in chars.chunks(2) {
            if chunk.len() == 2 {
                let hex = format!("{}{}", chunk[0], chunk[1]);
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    let decoded_char = (byte ^ 0x38) as char;
                    decoded_xor.push(decoded_char);
                }
            }
        }
        if decoded_xor.starts_with("/api")
            || decoded_xor.starts_with("http")
            || decoded_xor.starts_with("clock")
            || decoded_xor.starts_with("/clock")
        {
            return decoded_xor;
        }

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

    fn normalize_provider_url(source_url: &str) -> String {
        let trimmed = source_url.trim();
        if trimmed.starts_with("--") {
            Self::decode_provider_id(trimmed)
        } else if trimmed.starts_with("http") || trimmed.starts_with('/') {
            trimmed
                .replace("\\u002F", "/")
                .replace("\\/", "/")
                .replace("\\u0026", "&")
                .replace("\\u003D", "=")
                .replace('\\', "")
        } else {
            Self::decode_provider_id(trimmed)
        }
    }

    fn parse_quality(label: &str) -> Option<u32> {
        Regex::new(r"(?i)(\d{3,4})p?")
            .ok()?
            .captures(label)
            .and_then(|captures| captures.get(1))
            .and_then(|matched| matched.as_str().parse::<u32>().ok())
    }

    fn absolute_url(base_url: &str, maybe_relative: &str) -> String {
        if maybe_relative.starts_with("http") {
            return maybe_relative.to_string();
        }

        if maybe_relative.starts_with("//") {
            return format!("https:{}", maybe_relative);
        }

        let base_without_file = base_url
            .rsplit_once('/')
            .map(|(prefix, _)| prefix)
            .unwrap_or(base_url);

        if maybe_relative.starts_with('/') {
            if let Ok(parsed) = url::Url::parse(base_url) {
                return format!(
                    "{}://{}{}",
                    parsed.scheme(),
                    parsed.host_str().unwrap_or_default(),
                    maybe_relative
                );
            }
        }

        format!(
            "{}/{}",
            base_without_file.trim_end_matches('/'),
            maybe_relative.trim_start_matches('/')
        )
    }

    fn best_candidate(mut candidates: Vec<StreamCandidate>) -> Option<StreamCandidate> {
        candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.quality.unwrap_or(0)));
        candidates.into_iter().next()
    }

    fn source_priority() -> &'static [&'static str] {
        &[
            "Yt-mp4", "S-Mp4", "S-mp4", "Uv-mp4", "Ak", "Default", "Luf-Mp4", "Mp4", "Ss-Hls",
            "Sl-mp4", "Fm-Hls", "Fm-mp4", "Ok", "Other", "Sup", "Uni",
        ]
    }

    fn referrer_for_source(url: &str, source_name: &str) -> &'static str {
        if url.contains("mp4upload.com") || source_name == "Mp4" {
            MP4UPLOAD_REFERRER
        } else {
            ALLANIME_REFERRER
        }
    }

    fn extract_mp4upload_url(html: &str) -> Option<String> {
        let patterns = [
            r#"(?s)(?:src|file):\s*["']([^"']+\.mp4[^"']*)["']"#,
            r#"(?s)<source[^>]+src=["']([^"']+\.mp4[^"']*)["']"#,
        ];

        patterns.iter().find_map(|pattern| {
            Regex::new(pattern)
                .ok()?
                .captures(html)
                .and_then(|captures| captures.get(1))
                .map(|matched| matched.as_str().replace("\\u0026", "&").replace("\\/", "/"))
        })
    }

    fn extract_okru_stream_url(html: &str) -> Option<String> {
        let encoded_options = Regex::new(r#"data-options=\"([^\"]+)\""#)
            .ok()?
            .captures(html)?
            .get(1)?
            .as_str();
        let decoded_options = encoded_options
            .replace("&quot;", "\"")
            .replace("&amp;", "&")
            .replace("&#39;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">");
        let options: serde_json::Value = serde_json::from_str(&decoded_options).ok()?;
        let metadata = options.pointer("/flashvars/metadata")?.as_str()?;
        let metadata: serde_json::Value = serde_json::from_str(metadata).ok()?;

        metadata["ondemandHls"]
            .as_str()
            .filter(|url| !url.is_empty())
            .or_else(|| {
                metadata["videos"]
                    .as_array()?
                    .iter()
                    .rev()
                    .find_map(|video| video["url"].as_str().filter(|url| !url.is_empty()))
            })
            .map(str::to_string)
    }

    fn parse_hls_master_playlist(
        playlist: &str,
        playlist_url: &str,
        referrer: &str,
    ) -> Vec<StreamCandidate> {
        let mut candidates = Vec::new();
        let resolution_re = Regex::new(r"RESOLUTION=\d+x(\d+)").ok();
        let mut pending_quality = None;

        for line in playlist
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            if line.starts_with("#EXT-X-STREAM-INF") {
                pending_quality = resolution_re
                    .as_ref()
                    .and_then(|re| re.captures(line))
                    .and_then(|captures| captures.get(1))
                    .and_then(|matched| matched.as_str().parse::<u32>().ok());
                continue;
            }

            if line.starts_with('#') {
                continue;
            }

            let url = Self::absolute_url(playlist_url, line);
            let label = pending_quality
                .map(|quality| format!("{}p", quality))
                .unwrap_or_else(|| "hls".to_string());
            candidates.push(
                StreamCandidate::new(url, label)
                    .with_quality(pending_quality)
                    .with_referrer(referrer),
            );
            pending_quality = None;
        }

        candidates
    }

    fn collect_provider_json_links(
        value: &serde_json::Value,
        referrer: &str,
        candidates: &mut Vec<StreamCandidate>,
        subtitles: &mut Vec<Subtitle>,
    ) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(link) = map.get("link").and_then(|value| value.as_str()) {
                    let label = map
                        .get("resolutionStr")
                        .and_then(|value| value.as_str())
                        .unwrap_or("auto");
                    if link.starts_with("http") {
                        candidates.push(
                            StreamCandidate::new(link.to_string(), label)
                                .with_quality(Self::parse_quality(label))
                                .with_referrer(referrer),
                        );
                    }
                }

                if let Some(url) = map.get("url").and_then(|value| value.as_str()) {
                    if url.starts_with("http") {
                        let label = map
                            .get("height")
                            .and_then(|value| value.as_u64())
                            .map(|height| format!("{}p", height))
                            .or_else(|| {
                                map.get("resolutionStr")
                                    .and_then(|value| value.as_str())
                                    .map(str::to_string)
                            })
                            .unwrap_or_else(|| {
                                if url.contains(".m3u8") {
                                    "hls".to_string()
                                } else {
                                    "auto".to_string()
                                }
                            });
                        candidates.push(
                            StreamCandidate::new(url.to_string(), label.clone())
                                .with_quality(Self::parse_quality(&label))
                                .with_referrer(referrer),
                        );
                    }
                }

                if let Some(src) = map.get("src").and_then(|value| value.as_str()) {
                    if src.starts_with("http") {
                        let language = map
                            .get("lang")
                            .or_else(|| map.get("label"))
                            .and_then(|value| value.as_str())
                            .unwrap_or("en")
                            .to_string();
                        subtitles.push(Subtitle {
                            language,
                            url: src.to_string(),
                        });
                    }
                }

                for child in map.values() {
                    Self::collect_provider_json_links(child, referrer, candidates, subtitles);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    Self::collect_provider_json_links(item, referrer, candidates, subtitles);
                }
            }
            _ => {}
        }
    }

    fn extract_referrer_from_json(value: &serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::Object(map) => {
                for key in ["Referer", "referer", "referrer"] {
                    if let Some(referrer) = map.get(key).and_then(|value| value.as_str()) {
                        return Some(referrer.to_string());
                    }
                }

                map.values().find_map(Self::extract_referrer_from_json)
            }
            serde_json::Value::Array(items) => {
                items.iter().find_map(Self::extract_referrer_from_json)
            }
            _ => None,
        }
    }

    fn parse_provider_response(
        response: &str,
        default_referrer: &str,
    ) -> (Vec<StreamCandidate>, Vec<Subtitle>) {
        let mut candidates = Vec::new();
        let mut subtitles = Vec::new();

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(response) {
            let referrer = Self::extract_referrer_from_json(&json)
                .unwrap_or_else(|| default_referrer.to_string());
            Self::collect_provider_json_links(&json, &referrer, &mut candidates, &mut subtitles);
            return (candidates, subtitles);
        }

        let link_re = Regex::new(r#""link"\s*:\s*"([^"]+)".*?"resolutionStr"\s*:\s*"([^"]+)""#);
        if let Ok(re) = link_re {
            for captures in re.captures_iter(response) {
                let Some(link) = captures.get(1).map(|m| m.as_str()) else {
                    continue;
                };
                let label = captures.get(2).map(|m| m.as_str()).unwrap_or("auto");
                candidates.push(
                    StreamCandidate::new(link.replace("\\/", "/"), label)
                        .with_quality(Self::parse_quality(label))
                        .with_referrer(default_referrer),
                );
            }
        }

        (candidates, subtitles)
    }

    fn decode_base64_url(value: &str) -> Result<Vec<u8>> {
        let mut normalized = value.replace('-', "+").replace('_', "/");
        while !normalized.len().is_multiple_of(4) {
            normalized.push('=');
        }
        base64::engine::general_purpose::STANDARD
            .decode(normalized)
            .context("Failed to decode base64url value")
    }

    fn decrypt_filemoon_payload(response: &str) -> Result<Vec<StreamCandidate>> {
        let json: serde_json::Value =
            serde_json::from_str(response).context("Failed to parse Filemoon response JSON")?;
        let iv = json["iv"].as_str().context("Missing Filemoon iv")?;
        let payload = json["payload"]
            .as_str()
            .context("Missing Filemoon payload")?;
        let key_parts = json["key_parts"]
            .as_array()
            .context("Missing Filemoon key_parts")?;

        let mut key = Vec::new();
        for part in key_parts {
            let part = part.as_str().context("Invalid Filemoon key part")?;
            key.extend(Self::decode_base64_url(part)?);
        }
        if key.len() != 32 {
            anyhow::bail!("Invalid Filemoon key length: {}", key.len());
        }

        let iv_bytes = Self::decode_base64_url(iv)?;
        if iv_bytes.len() != 12 {
            anyhow::bail!("Invalid Filemoon iv length: {}", iv_bytes.len());
        }
        let mut ctr_iv = [0u8; 16];
        ctr_iv[..12].copy_from_slice(&iv_bytes);
        ctr_iv[15] = 2;

        let mut ciphertext = Self::decode_base64_url(payload)?;
        if ciphertext.len() <= 16 {
            anyhow::bail!("Invalid Filemoon payload length");
        }
        ciphertext.truncate(ciphertext.len() - 16);

        type Aes256Ctr = ctr::Ctr128BE<aes::Aes256>;
        let mut cipher = Aes256Ctr::new((&key[..]).into(), &ctr_iv.into());
        cipher.apply_keystream(&mut ciphertext);
        let plain = String::from_utf8(ciphertext).context("Invalid Filemoon UTF-8 payload")?;
        let value: serde_json::Value =
            serde_json::from_str(&plain).context("Failed to parse Filemoon decrypted JSON")?;

        let mut candidates = Vec::new();
        let mut subtitles = Vec::new();
        Self::collect_provider_json_links(
            &value,
            ALLANIME_REFERRER,
            &mut candidates,
            &mut subtitles,
        );
        Ok(candidates)
    }

    async fn resolve_direct_url(
        &self,
        url: &str,
        source_name: &str,
    ) -> Result<(Vec<StreamCandidate>, Vec<Subtitle>)> {
        if url.contains("mp4upload.com") && !url.contains(".mp4") {
            let html = self
                .insecure_client
                .get(url)
                .header(header::REFERER, ALLANIME_REFERRER)
                .send()
                .await
                .context("Failed to fetch mp4upload page")?
                .text()
                .await
                .context("Failed to read mp4upload page")?;

            if let Some(mp4_url) = Self::extract_mp4upload_url(&html) {
                return Ok((
                    vec![StreamCandidate::new(mp4_url, "mp4upload")
                        .with_referrer(MP4UPLOAD_REFERRER)],
                    Vec::new(),
                ));
            }

            tracing::warn!("AllAnime mp4upload embed did not contain a media URL");
            return Ok((Vec::new(), Vec::new()));
        }

        if url.contains("ok.ru/videoembed") {
            let html = self
                .client
                .get(url)
                .header(header::REFERER, ALLANIME_REFERRER)
                .send()
                .await
                .context("Failed to fetch OK.ru embed page")?
                .text()
                .await
                .context("Failed to read OK.ru embed page")?;

            if let Some(stream_url) = Self::extract_okru_stream_url(&html) {
                return Ok((
                    vec![StreamCandidate::new(stream_url, "ok.ru").with_referrer(url)],
                    Vec::new(),
                ));
            }

            tracing::warn!("AllAnime OK.ru embed did not contain a media URL");
            return Ok((Vec::new(), Vec::new()));
        }

        if url.contains(".m3u8") {
            let playlist = self
                .client
                .get(url)
                .header(header::REFERER, ALLANIME_REFERRER)
                .send()
                .await
                .context("Failed to fetch HLS playlist")?
                .text()
                .await
                .context("Failed to read HLS playlist")?;
            let candidates = Self::parse_hls_master_playlist(&playlist, url, ALLANIME_REFERRER);
            if !candidates.is_empty() {
                return Ok((candidates, Vec::new()));
            }
        }

        if source_name.starts_with("Fm") {
            let response = self
                .client
                .get(url)
                .header(header::REFERER, ALLANIME_REFERRER)
                .send()
                .await
                .context("Failed to fetch Fm-Hls page")?
                .text()
                .await
                .context("Failed to read Fm-Hls page")?;

            if let Ok(candidates) = Self::decrypt_filemoon_payload(&response) {
                if !candidates.is_empty() {
                    return Ok((candidates, Vec::new()));
                }
            }
        }

        let referrer = Self::referrer_for_source(url, source_name);

        Ok((
            vec![StreamCandidate::new(url.to_string(), source_name).with_referrer(referrer)],
            Vec::new(),
        ))
    }

    async fn resolve_provider_endpoint(
        &self,
        path: &str,
        source_name: &str,
    ) -> Result<(Vec<StreamCandidate>, Vec<Subtitle>)> {
        let endpoint = if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{}", ALLANIME_BASE, path)
        };

        let response = self
            .client
            .get(&endpoint)
            .header(header::REFERER, ALLANIME_REFERRER)
            .send()
            .await
            .with_context(|| format!("Failed to fetch AllAnime provider endpoint: {}", endpoint))?
            .text()
            .await
            .context("Failed to read AllAnime provider endpoint")?;

        if source_name.starts_with("Fm") {
            if let Ok(candidates) = Self::decrypt_filemoon_payload(&response) {
                if !candidates.is_empty() {
                    return Ok((candidates, Vec::new()));
                }
            }
        }

        let (mut candidates, subtitles) =
            Self::parse_provider_response(&response, ALLANIME_REFERRER);
        let mut expanded = Vec::new();
        for candidate in candidates.drain(..) {
            if candidate.url.contains(".m3u8") {
                let playlist = self
                    .client
                    .get(&candidate.url)
                    .header(
                        header::REFERER,
                        candidate
                            .headers
                            .get("Referer")
                            .map(String::as_str)
                            .unwrap_or(ALLANIME_REFERRER),
                    )
                    .send()
                    .await
                    .context("Failed to fetch provider HLS playlist")?
                    .text()
                    .await
                    .context("Failed to read provider HLS playlist")?;
                let referrer = candidate
                    .headers
                    .get("Referer")
                    .map(String::as_str)
                    .unwrap_or(ALLANIME_REFERRER);
                let variants = Self::parse_hls_master_playlist(&playlist, &candidate.url, referrer);
                if variants.is_empty() {
                    expanded.push(candidate);
                } else {
                    expanded.extend(variants);
                }
            } else {
                expanded.push(candidate);
            }
        }

        Ok((expanded, subtitles))
    }

    async fn resolve_source_url(
        &self,
        source_url: &str,
        source_name: &str,
    ) -> Result<(Vec<StreamCandidate>, Vec<Subtitle>)> {
        let normalized = Self::normalize_provider_url(source_url);
        if normalized.starts_with("http") {
            self.resolve_direct_url(&normalized, source_name).await
        } else if normalized.starts_with('/') {
            self.resolve_provider_endpoint(&normalized, source_name)
                .await
        } else {
            Ok((Vec::new(), Vec::new()))
        }
    }

    async fn candidate_is_playable(&self, candidate: &StreamCandidate) -> bool {
        let client = if candidate.url.contains("mp4upload.com")
            || candidate.headers.get("Referer").map(String::as_str) == Some(MP4UPLOAD_REFERRER)
        {
            &self.insecure_client
        } else {
            &self.client
        };

        let mut request = client.get(&candidate.url);
        if let Some(referrer) = candidate.headers.get("Referer") {
            request = request.header(header::REFERER, referrer);
        }
        if let Some(user_agent) = candidate.headers.get("User-Agent") {
            request = request.header(header::USER_AGENT, user_agent);
        }
        if !candidate.url.contains(".m3u8") {
            request = request.header(header::RANGE, "bytes=0-0");
        }

        match request.send().await {
            Ok(response)
                if response.status().is_success() || response.status().is_redirection() =>
            {
                let content_type = response
                    .headers()
                    .get(header::CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or_default()
                    .to_ascii_lowercase();

                if content_type.contains("text/html") {
                    tracing::warn!(
                        host = %Self::url_host(&candidate.url),
                        "AllAnime candidate probe resolved to HTML instead of media"
                    );
                    return false;
                }

                true
            }
            Ok(response) => {
                tracing::warn!(
                    host = %Self::url_host(&candidate.url),
                    status = %response.status(),
                    "AllAnime candidate failed probe"
                );
                false
            }
            Err(err) => {
                tracing::warn!(
                    host = %Self::url_host(&candidate.url),
                    error = %err,
                    "AllAnime candidate probe failed"
                );
                false
            }
        }
    }

    fn url_host(value: &str) -> String {
        url::Url::parse(value)
            .ok()
            .and_then(|url| url.host_str().map(str::to_string))
            .unwrap_or_else(|| "unknown-host".to_string())
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
        let mut results = Vec::new();
        let provider_queries = vec![query];

        for provider_query in provider_queries {
            let variables = serde_json::json!({
                "search": {
                    "query": provider_query,
                    "listProfile": "browse"
                },
                "limit": 26,
                "page": 1,
                "translationType": "sub"
            });
            let response = self
                .graphql_query(ALLANIME_QUERY_HASH_SEARCH, variables)
                .await?;
            let shows = if let Some(data) = response.get("data") {
                &data["shows"]
            } else {
                &response["shows"]
            };

            if let Some(edges) = shows["edges"].as_array() {
                for edge in edges {
                    let id = edge["_id"].as_str().unwrap_or_default().to_string();
                    let name = canonical_allanime_title(edge["name"].as_str().unwrap_or_default());
                    let thumbnail = edge["thumbnail"].as_str().unwrap_or_default().to_string();
                    let episodes = edge["availableEpisodes"]["sub"].as_u64().map(|n| n as u32);

                    if !id.is_empty()
                        && !name.is_empty()
                        && !results.iter().any(|anime: &Anime| anime.id == id)
                    {
                        results.push(Anime {
                            id,
                            provider: "AllAnime".to_string(),
                            title: name,
                            cover_url: thumbnail.clone(),
                            language: Language::English,
                            total_episodes: episodes,
                            synopsis: None,
                            anilist_id: anilist_id_from_thumbnail(&thumbnail),
                        });
                    }
                }
            }
        }

        if normalized_title(query) == "one piece"
            && !results
                .iter()
                .any(|anime| anime.title == "One Piece" && anime.total_episodes.unwrap_or(0) > 100)
        {
            // The upstream search index currently omits its abbreviated `1P`
            // record even though direct details and playback remain healthy.
            if let Some(anime) = allanime_details(self, "ReooPAxPMsHM4KPMY").await? {
                results.push(anime);
            }
        }

        results.sort_by(|left, right| {
            allanime_search_score(query, right)
                .cmp(&allanime_search_score(query, left))
                .then_with(|| right.total_episodes.cmp(&left.total_episodes))
        });
        Ok(results)
    }

    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        let variables = serde_json::json!({
            "_id": anime_id,
            "search": { "fromSearch": true }
        });

        let response = self
            .graphql_query(ALLANIME_QUERY_HASH_DETAIL, variables)
            .await?;
        let mut episodes = Vec::new();

        let show = if let Some(data) = response.get("data") {
            &data["show"]
        } else {
            &response["show"]
        };

        if let Some(episode_list) = show["availableEpisodesDetail"]["sub"].as_array() {
            for (idx, ep) in episode_list.iter().enumerate() {
                if let Some(ep_num_str) = ep.as_str() {
                    let ep_number = parse_episode_number(ep_num_str);
                    episodes.push(Episode {
                        id: format!("{}:{}", anime_id, ep_num_str),
                        number: if ep_number == 0 {
                            (idx + 1) as u32
                        } else {
                            ep_number
                        },
                        title: None,
                        thumbnail: None,
                        aniskip_episode_number: (ep_number > 0).then_some(ep_number),
                    });
                }
            }
        }

        episodes.sort_by_key(|a| a.number);
        Ok(episodes)
    }

    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo> {
        let parts: Vec<&str> = episode_id.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid episode_id format. Expected 'anime_id:episode_number'");
        }

        let anime_id = parts[0];
        let episode_number = parts[1];

        let variables = serde_json::json!({
            "showId": anime_id,
            "translationType": "sub",
            "episodeString": episode_number
        });

        let mut source_response = None;
        let mut response_key = None;
        for attempt in 0..2 {
            let (crypto, aa_request) = self.build_source_request_crypto().await?;
            let extensions = serde_json::json!({
                "persistedQuery": {
                    "version": 1,
                    "sha256Hash": ALLANIME_QUERY_HASH_EPISODE
                },
                "k": ALLANIME_LANE,
                "aaReq": aa_request
            });
            let encoded_vars =
                url::form_urlencoded::byte_serialize(variables.to_string().as_bytes())
                    .collect::<String>();
            let encoded_ext =
                url::form_urlencoded::byte_serialize(extensions.to_string().as_bytes())
                    .collect::<String>();
            let api_url = format!(
                "{}?variables={}&extensions={}",
                ALLANIME_API, encoded_vars, encoded_ext
            );
            let response: serde_json::Value = self
                .client
                .get(&api_url)
                .header("Origin", ALLANIME_ORIGIN)
                .header("x-build-id", ALLANIME_BUILD_ID)
                .send()
                .await
                .context("AllAnime source GraphQL request failed")?
                .json()
                .await
                .context("Failed to parse AllAnime source response")?;
            let crypto_error = response
                .get("errors")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|errors| {
                    errors.iter().any(|error| {
                        error["message"]
                            .as_str()
                            .is_some_and(|message| message.starts_with("AA_CRYPTO_"))
                    })
                });
            if crypto_error && attempt == 0 {
                *self.crypto.write().await = None;
                continue;
            }

            response_key = Some(crypto.key);
            source_response = Some(response);
            break;
        }
        let response = source_response.context("AllAnime source request returned no response")?;
        let response_key =
            response_key.context("AllAnime source request returned no crypto key")?;

        // Check if data is wrapped in tobeparsed
        let final_json = match response
            .get("data")
            .and_then(|d| d.get("tobeparsed"))
            .and_then(|t| t.as_str())
        {
            Some(tobeparsed) => {
                let decrypted = Self::decrypt_source_payload(tobeparsed, &response_key)?;
                serde_json::from_str::<serde_json::Value>(&decrypted)
                    .context("Failed to parse decrypted JSON")?
            }
            None => response,
        };

        if final_json
            .to_string()
            .to_ascii_lowercase()
            .contains("need_captcha")
        {
            anyhow::bail!("PROVIDER_CAPTCHA: NEED_CAPTCHA");
        }

        let mut subtitles = Vec::new();
        let mut selected_candidate = None;

        let episode = if let Some(data) = final_json.get("data") {
            &data["episode"]
        } else {
            &final_json["episode"]
        };

        if let Some(source_urls) = episode["sourceUrls"].as_array() {
            let available_sources = source_urls
                .iter()
                .filter_map(|source| source["sourceName"].as_str())
                .collect::<Vec<_>>();
            tracing::debug!(
                sources = ?available_sources,
                anime_id,
                episode_number,
                "AllAnime episode sources received"
            );

            for &priority_name in Self::source_priority() {
                let Some(source) = source_urls
                    .iter()
                    .find(|s| s["sourceName"].as_str() == Some(priority_name))
                else {
                    continue;
                };

                if let Some(source_url) = source["sourceUrl"].as_str() {
                    match self.resolve_source_url(source_url, priority_name).await {
                        Ok((mut resolved, mut resolved_subtitles)) => {
                            tracing::debug!(
                                source = priority_name,
                                candidate_count = resolved.len(),
                                "AllAnime source resolved"
                            );
                            subtitles.append(&mut resolved_subtitles);
                            while let Some(candidate) = Self::best_candidate(resolved.clone()) {
                                let playable = self.candidate_is_playable(&candidate).await;
                                tracing::debug!(
                                    source = priority_name,
                                    playable,
                                    "AllAnime candidate probed"
                                );
                                if playable {
                                    selected_candidate = Some(candidate);
                                    break;
                                }

                                resolved.retain(|item| item.url != candidate.url);
                            }

                            if selected_candidate.is_some() {
                                break;
                            }
                        }
                        Err(err) => {
                            tracing::warn!(
                                source = priority_name,
                                anime_id,
                                episode_number,
                                error = %err,
                                "AllAnime source failed"
                            );
                        }
                    }
                }
            }
        }

        let Some(candidate) = selected_candidate else {
            tracing::warn!(
                anime_id,
                episode_number,
                "AllAnime returned no working stream"
            );
            anyhow::bail!(
                "No working stream URL found. This might be a temporary issue with AllAnime."
            );
        };

        Ok(StreamInfo {
            video_url: candidate.url,
            subtitles,
            qualities: vec![candidate.label],
            headers: candidate.headers,
            use_curl: false,
        })
    }
}

async fn allanime_details(provider: &AllAnimeProvider, anime_id: &str) -> Result<Option<Anime>> {
    let variables = serde_json::json!({
        "_id": anime_id,
        "search": { "fromSearch": true }
    });

    let response = provider
        .graphql_query(ALLANIME_QUERY_HASH_DETAIL, variables)
        .await?;
    let show = if let Some(data) = response.get("data") {
        &data["show"]
    } else {
        &response["show"]
    };

    let id = show["_id"].as_str().unwrap_or(anime_id).to_string();
    let title = canonical_allanime_title(show["name"].as_str().unwrap_or_default());
    if title.is_empty() {
        return Ok(None);
    }

    let cover_url = show["thumbnail"].as_str().unwrap_or_default().to_string();
    let total_episodes = show["availableEpisodes"]["sub"].as_u64().map(|n| n as u32);
    let synopsis = show["description"].as_str().map(str::to_string);

    Ok(Some(Anime {
        id,
        provider: "AllAnime".to_string(),
        title,
        cover_url: cover_url.clone(),
        language: Language::English,
        total_episodes,
        synopsis,
        anilist_id: anilist_id_from_thumbnail(&cover_url),
    }))
}

fn anilist_id_from_thumbnail(cover_url: &str) -> Option<i64> {
    // AllAnime hosts covers on AniList's CDN: .../bx{anilist_id}-....webp
    let segment = cover_url.rsplit('/').find(|part| part.starts_with("bx"))?;
    segment
        .strip_prefix("bx")?
        .split(|character: char| !character.is_ascii_digit())
        .next()?
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
}

fn normalized_title(value: &str) -> String {
    value
        .chars()
        .filter_map(|character| {
            if character.is_ascii_alphanumeric() {
                Some(character.to_ascii_lowercase())
            } else if character.is_whitespace() {
                Some(' ')
            } else {
                None
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn canonical_allanime_title(value: &str) -> String {
    // AllAnime currently abbreviates its main One Piece entry as `1P` while
    // still returning the complete 1,100+ episode catalogue. Normalize that
    // provider-owned alias so search ranking and the user-visible title select
    // the playable series instead of a one-episode special.
    if value.trim().eq_ignore_ascii_case("1P") {
        "One Piece".to_string()
    } else {
        value.to_string()
    }
}

fn allanime_search_score(query: &str, anime: &Anime) -> i32 {
    let query = normalized_title(query);
    let title = normalized_title(&anime.title);
    let mut score = if title == query {
        1000
    } else if title.starts_with(&query) {
        650
    } else if title.contains(&query) {
        350
    } else {
        0
    };

    let total_episodes = anime.total_episodes.unwrap_or_default();
    if total_episodes > 100 {
        score += 250;
    } else if total_episodes > 20 {
        score += 80;
    }

    if !(query.contains("movie") || query.contains("film") || query.contains("special")) {
        let special_terms = ["movie", "film", "special", "episode of"];
        if special_terms.iter().any(|term| title.contains(term)) {
            score -= 180;
        }
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    #[test]
    fn test_decode_provider_id() {
        let encoded = "79677a7a78";
        let decoded = AllAnimeProvider::decode_provider_id(encoded);
        assert!(!decoded.is_empty());
    }

    #[test]
    fn normalizes_current_one_piece_provider_alias() {
        assert_eq!(canonical_allanime_title("1P"), "One Piece");
        assert_eq!(canonical_allanime_title("Your Name"), "Your Name");
    }

    #[test]
    fn test_normalize_preserves_direct_url() {
        let direct = "https://www.mp4upload.com/embed-abc123.html";
        assert_eq!(AllAnimeProvider::normalize_provider_url(direct), direct);
    }

    #[test]
    fn parses_anilist_id_from_cdn_thumbnail() {
        let url =
            "https://s4.anilist.co/file/anilistcdn/media/anime/cover/large/bx21-GG9Wn0rY5vDq.png";
        assert_eq!(anilist_id_from_thumbnail(url), Some(21));
        assert_eq!(
            anilist_id_from_thumbnail("https://img.example.com/poster.webp"),
            None
        );
        assert_eq!(anilist_id_from_thumbnail(""), None);
    }

    #[test]
    fn test_extract_mp4upload_url() {
        let html = r#"
            <script>
              player.setup({
                file: "https://s1.mp4upload.com:282/d/example/video.mp4?token=a\u0026b=c"
              });
            </script>
        "#;

        assert_eq!(
            AllAnimeProvider::extract_mp4upload_url(html).as_deref(),
            Some("https://s1.mp4upload.com:282/d/example/video.mp4?token=a&b=c")
        );
    }

    #[test]
    fn test_extract_okru_hls_url() {
        let html = r#"<div data-options="{&quot;flashvars&quot;:{&quot;metadata&quot;:&quot;{\&quot;videos\&quot;:[{\&quot;url\&quot;:\&quot;https://cdn.example/video.mp4\&quot;}],\&quot;ondemandHls\&quot;:\&quot;https://cdn.example/master.m3u8\&quot;}&quot;}}"></div>"#;
        assert_eq!(
            AllAnimeProvider::extract_okru_stream_url(html).as_deref(),
            Some("https://cdn.example/master.m3u8")
        );
    }

    #[test]
    fn test_parse_hls_master_playlist() {
        let playlist = r#"#EXTM3U
#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360
360/index.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=2200000,RESOLUTION=1280x720
https://cdn.example/720/index.m3u8
"#;

        let candidates = AllAnimeProvider::parse_hls_master_playlist(
            playlist,
            "https://cdn.example/master.m3u8",
            "https://referrer.example/",
        );

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].quality, Some(360));
        assert_eq!(
            candidates[0].url,
            "https://cdn.example/360/index.m3u8".to_string()
        );
        assert_eq!(candidates[1].quality, Some(720));
    }

    #[test]
    fn test_best_candidate_prefers_highest_quality() {
        let candidates = vec![
            StreamCandidate::new("https://cdn.example/360.m3u8".to_string(), "360p")
                .with_quality(Some(360)),
            StreamCandidate::new("https://cdn.example/1080.m3u8".to_string(), "1080p")
                .with_quality(Some(1080)),
        ];

        let best = AllAnimeProvider::best_candidate(candidates).unwrap();
        assert_eq!(best.quality, Some(1080));
    }

    #[test]
    fn test_source_priority_covers_current_allanime_sources() {
        let priority = AllAnimeProvider::source_priority();

        assert_eq!(&priority[..5], ["Yt-mp4", "S-Mp4", "S-mp4", "Uv-mp4", "Ak"]);
        assert!(priority.contains(&"Default"));
        assert!(priority.contains(&"Fm-Hls"));
        assert!(priority.contains(&"Fm-mp4"));
        assert!(priority.contains(&"Luf-Mp4"));
    }

    #[test]
    fn test_build_mask_matches_live_runtime_vector() {
        let mask = AllAnimeProvider::build_mask("110");
        let expected = "43809dde8bfd864b69b1df370ffed8ae9df9e42b003718904109d37bc59d8d2d";
        let actual = mask
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_boot_token_matches_live_bootstrap_vector() {
        let mask = AllAnimeProvider::build_mask("110");
        let token = AllAnimeProvider::boot_token(&mask, "110", "mkissa", "mkissa.to", 2953, "k7");
        assert_eq!(
            token,
            "f7144f84383621714b5162b7154e0e0fd3f235a39cc5f8deaa0c9efdab2cd064"
        );
    }

    #[test]
    fn test_current_epoch_matches_server_epoch() {
        let now = AllAnimeProvider::unix_time_ms();
        let epoch = AllAnimeProvider::current_epoch();
        assert!(now / 604_800_000 >= epoch);
        assert!(epoch + 1 >= now / 604_800_000);
    }

    #[test]
    fn test_decrypt_source_payload_aes_gcm_fixture() {
        let key = [7u8; 32];
        let iv = [3u8; 12];
        let plain = br#"{"episode":{"sourceUrls":[]}}"#;
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let encrypted = cipher
            .encrypt(Nonce::from_slice(&iv), plain.as_ref())
            .unwrap();
        let mut payload = vec![1];
        payload.extend_from_slice(&iv);
        payload.extend_from_slice(&encrypted);

        let decoded = AllAnimeProvider::decrypt_source_payload(
            &base64::engine::general_purpose::STANDARD.encode(payload),
            &key,
        )
        .unwrap();
        assert_eq!(decoded.as_bytes(), plain);
    }

    #[test]
    fn test_mp4upload_referrer_matches_upstream_flag() {
        assert_eq!(
            AllAnimeProvider::referrer_for_source(
                "https://www.mp4upload.com/embed-example.html",
                "Mp4",
            ),
            "https://www.mp4upload.com"
        );
    }

    #[test]
    fn search_score_prefers_main_long_running_series() {
        let special = Anime {
            id: "special".into(),
            provider: "AllAnime".into(),
            title: "One Piece: Episode of Skypiea".into(),
            cover_url: String::new(),
            language: Language::English,
            total_episodes: Some(1),
            synopsis: None,
            anilist_id: None,
        };
        let series = Anime {
            id: "series".into(),
            provider: "AllAnime".into(),
            title: "One Piece".into(),
            cover_url: String::new(),
            language: Language::English,
            total_episodes: Some(1167),
            synopsis: None,
            anilist_id: None,
        };

        assert!(
            allanime_search_score("One Piece", &series)
                > allanime_search_score("One Piece", &special)
        );
    }

    #[test]
    fn test_parse_provider_response_collects_links_and_subtitles() {
        let response = r#"{
            "links": [
                {"link": "https://cdn.example/720.mp4", "resolutionStr": "720p"},
                {"hls": true, "url": "https://cdn.example/master.m3u8"}
            ],
            "subtitles": [
                {"lang": "en", "src": "https://cdn.example/subs.vtt"}
            ],
            "Referer": "https://provider.example/"
        }"#;

        let (candidates, subtitles) =
            AllAnimeProvider::parse_provider_response(response, ALLANIME_REFERRER);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].quality, Some(720));
        assert_eq!(
            candidates[0].headers.get("Referer").map(String::as_str),
            Some("https://provider.example/")
        );
        assert_eq!(subtitles.len(), 1);
        assert_eq!(subtitles[0].language, "en");
    }

    #[test]
    fn test_decrypt_filemoon_payload_fixture() {
        let key: Vec<u8> = (0..32).collect();
        let iv: Vec<u8> = (32..44).collect();
        let plaintext = br#"[{"url":"https://cdn.example/filemoon-720.mp4","height":720}]"#;
        let mut encrypted = plaintext.to_vec();

        let mut ctr_iv = [0u8; 16];
        ctr_iv[..12].copy_from_slice(&iv);
        ctr_iv[15] = 2;

        type Aes256Ctr = ctr::Ctr128BE<aes::Aes256>;
        let mut cipher = Aes256Ctr::new((&key[..]).into(), &ctr_iv.into());
        cipher.apply_keystream(&mut encrypted);
        encrypted.extend([0u8; 16]);

        let response = serde_json::json!({
            "iv": URL_SAFE_NO_PAD.encode(&iv),
            "payload": URL_SAFE_NO_PAD.encode(&encrypted),
            "key_parts": [
                URL_SAFE_NO_PAD.encode(&key[..16]),
                URL_SAFE_NO_PAD.encode(&key[16..])
            ]
        });

        let candidates = AllAnimeProvider::decrypt_filemoon_payload(&response.to_string()).unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].url, "https://cdn.example/filemoon-720.mp4");
        assert_eq!(candidates[0].quality, Some(720));
    }
}
