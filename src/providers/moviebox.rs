use super::{Anime, AnimeProvider, Episode, Language, StreamInfo, Subtitle};
use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use reqwest::{Method, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const API_BASES: [&str; 3] = [
    "https://api4.aoneroom.com",
    "https://api5.aoneroom.com",
    "https://api6.aoneroom.com",
];
const SECRET: &str = "NzZpUmwwN3MweFNOOWpxbUVXQXQ3OUVCSlp1bElRSXNWNjRGWnIyTw==";
const MOBILE_USER_AGENT: &str = "com.community.oneroom/50020052 (Linux; U; Android 16; en_IN; sdk_gphone64_x86_64; Build/BP22.250325.006; Cronet/133.0.6876.3)";
const PLAYBACK_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/124 Safari/537.36";

type HmacMd5 = Hmac<Md5>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MovieBoxEpisode {
    subject_id: String,
    season: u32,
    episode: u32,
}

pub struct MovieBoxProvider {
    client: reqwest::Client,
    tokens: tokio::sync::Mutex<HashMap<String, String>>,
}

impl Default for MovieBoxProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MovieBoxProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .expect("failed to build MovieBox client"),
            tokens: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    async fn get_token(&self, api_base: &str) -> Result<String> {
        if let Some(token) = self.tokens.lock().await.get(api_base).cloned() {
            return Ok(token);
        }

        let url = Url::parse(api_base)?
            .join("/wefeed-mobile-bff/tab-operating?page=1&tabId=0&version=")?;
        let headers = signed_headers(&Method::GET, &url, None, None, None)?;
        let response = self
            .client
            .request(Method::GET, url)
            .headers(headers)
            .send()
            .await
            .context("MovieBox auth request failed")?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("PROVIDER_UNAVAILABLE: MovieBox auth failed (HTTP {status})");
        }

        let x_user = response
            .headers()
            .get("x-user")
            .context("MovieBox auth response missing x-user header")?
            .to_str()
            .context("MovieBox auth x-user header is not valid UTF-8")?;

        let x_user_json: Value =
            serde_json::from_str(x_user).context("MovieBox auth x-user header is invalid JSON")?;

        let token = x_user_json["token"]
            .as_str()
            .context("MovieBox auth x-user header missing token")?
            .to_string();

        self.tokens
            .lock()
            .await
            .insert(api_base.to_string(), token.clone());
        Ok(token)
    }

    async fn request_json(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        play_mode: Option<&str>,
    ) -> Result<Value> {
        let body_text = body
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .context("failed to encode MovieBox request")?;
        let mut failures = Vec::new();
        for api_base in API_BASES {
            match self
                .request_json_at(api_base, &method, path, body_text.as_deref(), play_mode)
                .await
            {
                Ok(value) => return Ok(value),
                Err(error) => failures.push(format!("{api_base}: {error:#}")),
            }
        }
        anyhow::bail!(
            "PROVIDER_UNAVAILABLE: every MovieBox API host failed: {}",
            failures.join("; ")
        )
    }

    async fn request_json_at(
        &self,
        api_base: &str,
        method: &Method,
        path: &str,
        body_text: Option<&str>,
        play_mode: Option<&str>,
    ) -> Result<Value> {
        let url = Url::parse(api_base)?.join(path)?;
        let token = self.get_token(api_base).await?;
        let headers = signed_headers(method, &url, body_text, play_mode, Some(&token))?;
        let mut request = self.client.request(method.clone(), url).headers(headers);
        if let Some(body_text) = body_text {
            request = request.body(body_text.to_string());
        }
        let response = request
            .send()
            .await
            .with_context(|| format!("MovieBox request failed through {api_base}"))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .context("MovieBox returned an unreadable response")?;
        let value: Value = serde_json::from_str(&text)
            .with_context(|| format!("MovieBox returned invalid JSON (HTTP {status})"))?;
        let code = value["code"].as_i64().unwrap_or_default();
        if !status.is_success() || code != 0 {
            let message = value["message"]
                .as_str()
                .or_else(|| value["msg"].as_str())
                .unwrap_or("MovieBox request failed");
            anyhow::bail!("PROVIDER_UNAVAILABLE: MovieBox HTTP {status}, code {code}: {message}");
        }
        Ok(value)
    }

    async fn details(&self, subject_id: &str) -> Result<Value> {
        self.request_json(
            Method::GET,
            &format!("/wefeed-mobile-bff/subject-api/get?subjectId={subject_id}"),
            None,
            Some("2"),
        )
        .await
    }

    fn subject(value: &Value) -> &Value {
        value["data"]["subject"]
            .as_object()
            .map(|_| &value["data"]["subject"])
            .unwrap_or(&value["data"])
    }

    fn english_subject_id(details: &Value) -> Option<String> {
        let subject = Self::subject(details);
        let main_language = subject["lanName"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase();
        if main_language.contains("english") && main_language.contains("sub") {
            return json_string(&subject["subjectId"]);
        }
        subject["dubs"].as_array()?.iter().find_map(|dub| {
            let language = dub["lanName"].as_str()?.to_lowercase();
            (language.contains("english") && language.contains("sub"))
                .then(|| json_string(&dub["subjectId"]))
                .flatten()
        })
    }

    fn parse_search_items(value: &Value) -> Vec<Anime> {
        let mut subjects = Vec::new();
        if let Some(groups) = value["data"]["results"].as_array() {
            for group in groups {
                if let Some(items) = group["subjects"].as_array() {
                    subjects.extend(items.iter());
                }
            }
        }
        for key in ["subjectList", "items", "subjects"] {
            if let Some(items) = value["data"][key].as_array() {
                subjects.extend(items.iter());
            }
        }

        let mut results = subjects
            .into_iter()
            .filter_map(|item| {
                let subject = item.get("subject").unwrap_or(item);
                let id =
                    json_string(&subject["subjectId"]).or_else(|| json_string(&subject["id"]))?;
                let title = subject["title"].as_str()?.trim().to_string();
                let genres = subject["genre"].as_str().unwrap_or_default().to_lowercase();
                let type_code = subject["subjectType"].as_i64().unwrap_or(1);
                if type_code != 1 && type_code != 2 && type_code != 100 {
                    return None;
                }
                if !genres.is_empty() && !genres.contains("anime") && !genres.contains("animation")
                {
                    return None;
                }
                Some(Anime {
                    id,
                    provider: "MovieBox".into(),
                    title,
                    cover_url: subject["cover"]["url"]
                        .as_str()
                        .or_else(|| subject["coverUrl"].as_str())
                        .unwrap_or_default()
                        .to_string(),
                    language: Language::English,
                    total_episodes: subject["resourceDetectors"]["totalEpisode"]
                        .as_u64()
                        .map(|value| value as u32),
                    synopsis: subject["description"].as_str().map(str::to_string),
                    anilist_id: None,
                })
            })
            .collect::<Vec<_>>();
        results.dedup_by(|left, right| left.id == right.id);
        results
    }
}

#[async_trait]
impl AnimeProvider for MovieBoxProvider {
    fn name(&self) -> &str {
        "MovieBox"
    }

    fn language(&self) -> Language {
        Language::English
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["en".into()]
    }

    async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        let value = self
            .request_json(
                Method::POST,
                "/wefeed-mobile-bff/subject-api/search/v2",
                Some(serde_json::json!({
                    "page": 1,
                    "perPage": 20,
                    "keyword": query,
                })),
                None,
            )
            .await?;
        Ok(Self::parse_search_items(&value))
    }

    async fn get_episodes(&self, anime_id: &str) -> Result<Vec<Episode>> {
        let details = self.details(anime_id).await?;
        let english_id = Self::english_subject_id(&details).context(
            "STREAM_NOT_FOUND: MovieBox has no certified English-sub edition for this title",
        )?;
        let seasons = self
            .request_json(
                Method::GET,
                &format!("/wefeed-mobile-bff/subject-api/season-info?subjectId={english_id}"),
                None,
                Some("2"),
            )
            .await?;
        let season_list = seasons["data"]["resource"]["seasons"]
            .as_array()
            .or_else(|| seasons["data"]["seasons"].as_array());
        let mut episodes = Vec::new();
        if let Some(season_list) = season_list {
            let mut ordinal = 1u32;
            for season in season_list {
                let season_number = json_u32(&season["se"]).unwrap_or(1);
                let episode_numbers = season["allEp"]
                    .as_str()
                    .map(|value| {
                        value
                            .split(',')
                            .filter_map(|part| part.trim().parse::<u32>().ok())
                            .collect::<Vec<_>>()
                    })
                    .filter(|items| !items.is_empty())
                    .unwrap_or_else(|| {
                        let maximum = json_u32(&season["maxEp"]).unwrap_or(1);
                        (1..=maximum).collect()
                    });
                for episode_number in episode_numbers {
                    let reference = MovieBoxEpisode {
                        subject_id: english_id.clone(),
                        season: season_number,
                        episode: episode_number,
                    };
                    episodes.push(Episode {
                        id: serde_json::to_string(&reference)?,
                        number: ordinal,
                        title: Some(if season_list.len() > 1 {
                            format!("Season {season_number}, Episode {episode_number}")
                        } else {
                            format!("Episode {episode_number}")
                        }),
                        thumbnail: None,
                        aniskip_episode_number: Some(episode_number),
                    });
                    ordinal += 1;
                }
            }
        }
        if episodes.is_empty() {
            episodes.push(Episode {
                id: serde_json::to_string(&MovieBoxEpisode {
                    subject_id: english_id,
                    season: 0,
                    episode: 0,
                })?,
                number: 1,
                title: Some("Play movie".into()),
                thumbnail: None,
                aniskip_episode_number: Some(1),
            });
        }
        Ok(episodes)
    }

    async fn get_stream_url(&self, episode_id: &str) -> Result<StreamInfo> {
        let episode: MovieBoxEpisode = serde_json::from_str(episode_id)
            .context("MovieBox received an invalid episode identifier")?;
        let value = self
            .request_json(
                Method::GET,
                &format!(
                    "/wefeed-mobile-bff/subject-api/play-info?subjectId={}&se={}&ep={}",
                    episode.subject_id, episode.season, episode.episode
                ),
                None,
                Some("1"),
            )
            .await?;
        let streams = value["data"]["streams"]
            .as_array()
            .context("STREAM_NOT_FOUND: MovieBox returned no streams")?;
        let stream = streams
            .iter()
            .filter(|stream| stream["url"].as_str().is_some())
            .max_by_key(|stream| stream_priority(stream["url"].as_str().unwrap_or_default()))
            .context("STREAM_NOT_FOUND: MovieBox returned no playable stream")?;
        let video_url = stream["url"]
            .as_str()
            .context("STREAM_NOT_FOUND: MovieBox returned an empty stream URL")?
            .to_string();
        let stream_id = json_string(&stream["id"]).unwrap_or_default();
        let mut subtitles = Vec::new();
        if !stream_id.is_empty() {
            if let Ok(captions) = self
                .request_json(
                    Method::GET,
                    &format!(
                        "/wefeed-mobile-bff/subject-api/get-stream-captions?subjectId={}&streamId={stream_id}",
                        episode.subject_id
                    ),
                    None,
                    Some("1"),
                )
                .await
            {
                if let Some(items) = captions["data"]["extCaptions"].as_array() {
                    subtitles.extend(items.iter().filter_map(|caption| {
                        Some(Subtitle {
                            language: caption["lanName"]
                                .as_str()
                                .or_else(|| caption["language"].as_str())
                                .unwrap_or("English")
                                .to_string(),
                            url: caption["url"].as_str()?.to_string(),
                        })
                    }));
                }
            }
        }
        let mut headers = HashMap::new();
        if video_url.to_lowercase().contains(".mpd") {
            // DASH manifests on sacdn require the desktop UA, the web Referer
            // and the CloudFront signing cookies.
            headers.insert("Referer".into(), "https://h5.aoneroom.com/".into());
            headers.insert("User-Agent".into(), PLAYBACK_USER_AGENT.into());
        } else {
            // Direct MP4s on hcdn3 bind their URL signature to the mobile app
            // UA and reject requests carrying a Referer header.
            headers.insert("User-Agent".into(), MOBILE_USER_AGENT.into());
        }
        if let Some(cookie) = stream["signCookie"].as_str() {
            if !cookie.is_empty() {
                headers.insert("Cookie".into(), cookie.to_string());
            }
        }
        let qualities = stream["resolutions"]
            .as_str()
            .unwrap_or("Auto")
            .split(',')
            .map(|quality| quality.trim().to_string())
            .collect();
        Ok(StreamInfo {
            video_url,
            subtitles,
            qualities,
            headers,
            use_curl: false,
        })
    }
}

fn signed_headers(
    method: &Method,
    url: &Url,
    body: Option<&str>,
    play_mode: Option<&str>,
    auth_token: Option<&str>,
) -> Result<reqwest::header::HeaderMap> {
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, CONTENT_TYPE, USER_AGENT};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_millis() as u64;
    let content_type = if method == Method::POST {
        "application/json; charset=utf-8"
    } else {
        "application/json"
    };
    let canonical = canonical_string(method, url, body, timestamp, content_type);
    let first = STANDARD.decode(SECRET)?;
    let encoded_key = String::from_utf8(first).context("invalid MovieBox signing key")?;
    let key = STANDARD.decode(encoded_key.trim())?;
    let mut mac = HmacMd5::new_from_slice(&key).context("invalid MovieBox HMAC key")?;
    mac.update(canonical.as_bytes());
    let signature = STANDARD.encode(mac.finalize().into_bytes());
    let token_hash = md5_hex(
        timestamp
            .to_string()
            .chars()
            .rev()
            .collect::<String>()
            .as_bytes(),
    );

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(MOBILE_USER_AGENT));
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_str(content_type)?);
    headers.insert(
        HeaderName::from_static("x-client-token"),
        HeaderValue::from_str(&format!("{timestamp},{token_hash}"))?,
    );
    headers.insert(
        HeaderName::from_static("x-tr-signature"),
        HeaderValue::from_str(&format!("{timestamp}|2|{signature}"))?,
    );
    headers.insert(
        HeaderName::from_static("x-client-info"),
        HeaderValue::from_str(&client_info())?,
    );
    headers.insert(
        HeaderName::from_static("x-client-status"),
        HeaderValue::from_static("0"),
    );
    if let Some(play_mode) = play_mode {
        headers.insert(
            HeaderName::from_static("x-play-mode"),
            HeaderValue::from_str(play_mode)?,
        );
    }
    if let Some(token) = auth_token {
        headers.insert(
            reqwest::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token))?,
        );
    }
    Ok(headers)
}

fn canonical_string(
    method: &Method,
    url: &Url,
    body: Option<&str>,
    timestamp: u64,
    content_type: &str,
) -> String {
    let mut query = url.query_pairs().into_owned().collect::<Vec<_>>();
    query.sort();
    let canonical_url = if query.is_empty() {
        url.path().to_string()
    } else {
        format!(
            "{}?{}",
            url.path(),
            query
                .into_iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join("&")
        )
    };
    let body_bytes = body.map(str::as_bytes);
    let body_hash = body_bytes.map(md5_hex).unwrap_or_default();
    let body_length = body_bytes
        .map(|bytes| bytes.len().to_string())
        .unwrap_or_default();
    format!(
        "{}\napplication/json\n{content_type}\n{body_length}\n{timestamp}\n{body_hash}\n{canonical_url}",
        method.as_str().to_uppercase()
    )
}

fn client_info() -> String {
    serde_json::json!({
        "package_name": "com.community.oneroom",
        "version_name": "3.0.05.0711.03",
        "version_code": 50020052,
        "os": "android",
        "os_version": "16",
        "device_id": "da2b99c821e6ea023e4be55b54d5f7d8",
        "install_store": "ps",
        "gaid": "d7578036d13336cc",
        "brand": "google",
        "model": "sdk_gphone64_x86_64",
        "system_language": "en",
        "net": "NETWORK_WIFI",
        "region": "IN",
        "timezone": "Asia/Calcutta",
        "sp_code": ""
    })
    .to_string()
}

fn md5_hex(input: &[u8]) -> String {
    let digest = Md5::digest(input);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn json_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.as_u64().map(|number| number.to_string()))
}

fn json_u32(value: &Value) -> Option<u32> {
    value
        .as_u64()
        .map(|number| number as u32)
        .or_else(|| value.as_str()?.parse().ok())
}

fn stream_priority(url: &str) -> u8 {
    let lowercase = url.to_lowercase();
    if lowercase.contains(".m3u8") {
        3
    } else if lowercase.contains(".mpd") {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_query_is_sorted() {
        let url = Url::parse("https://api3.aoneroom.com/play?subjectId=123&se=1&ep=2").unwrap();
        let canonical = canonical_string(&Method::GET, &url, None, 100, "application/json");
        assert!(canonical.ends_with("/play?ep=2&se=1&subjectId=123"));
    }

    #[test]
    fn parses_grouped_search_results() {
        let value = serde_json::json!({
            "data": {"results": [{"subjects": [{
                "subjectId": "42",
                "title": "Your Name",
                "genre": "Anime, Romance",
                "subjectType": 1,
                "cover": {"url": "https://example.com/cover.jpg"}
            }]}]}
        });
        let results = MovieBoxProvider::parse_search_items(&value);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "42");
    }
}
