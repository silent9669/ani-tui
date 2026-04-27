use anyhow::{Context, Result};
use semver::Version;
use std::env;

pub const REPO_OWNER: &str = "silent9669";
pub const REPO_NAME: &str = "ani-tui";
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InstallMethod {
    Homebrew,
    Scoop,
    Binary,
}

pub struct UpdateChecker {
    client: reqwest::Client,
}

impl UpdateChecker {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .user_agent(format!("ani-tui/{}", CURRENT_VERSION))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    pub async fn check(&self) -> Result<Option<UpdateCheckResult>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            REPO_OWNER, REPO_NAME
        );

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .context("Failed to fetch release info")?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let json: serde_json::Value = response.json().await.context("Failed to parse response")?;

        let tag_name = json["tag_name"]
            .as_str()
            .unwrap_or("")
            .trim_start_matches('v');
        let html_url = json["html_url"].as_str().unwrap_or("").to_string();

        let current = Version::parse(CURRENT_VERSION).context("Failed to parse current version")?;
        let latest = Version::parse(tag_name).context("Failed to parse latest version")?;

        Ok(Some(UpdateCheckResult {
            current_version: CURRENT_VERSION.to_string(),
            latest_version: tag_name.to_string(),
            has_update: latest > current,
            release_url: html_url,
        }))
    }

    pub fn detect_install_method() -> InstallMethod {
        if Self::is_homebrew_install() {
            return InstallMethod::Homebrew;
        }

        if Self::is_scoop_install() {
            return InstallMethod::Scoop;
        }

        InstallMethod::Binary
    }

    fn is_homebrew_install() -> bool {
        if let Ok(prefix) = env::var("HOMEBREW_PREFIX") {
            if let Ok(exe_path) = env::current_exe() {
                if exe_path.starts_with(&prefix) {
                    return true;
                }
            }
        }

        if let Ok(exe_path) = env::current_exe() {
            let homebrew_prefixes = [
                "/opt/homebrew/bin/",
                "/opt/homebrew/Cellar/",
                "/usr/local/bin/",
                "/usr/local/Cellar/",
                "/home/linuxbrew/.linuxbrew/bin/",
                "/home/linuxbrew/.linuxbrew/Cellar/",
            ];
            for prefix in homebrew_prefixes {
                if exe_path.starts_with(prefix) {
                    return true;
                }
            }
        }

        false
    }

    #[cfg(windows)]
    fn is_scoop_install() -> bool {
        if let Ok(scoop) = env::var("SCOOP") {
            if let Ok(exe_path) = env::current_exe() {
                if exe_path.starts_with(&scoop) {
                    return true;
                }
            }
        }

        if let Ok(exe_path) = env::current_exe() {
            if let Ok(userprofile) = env::var("USERPROFILE") {
                let scoop_paths = [
                    format!("{}\\scoop\\apps", userprofile),
                    format!("{}\\scoop", userprofile),
                ];
                for path in scoop_paths {
                    if exe_path.starts_with(&path) {
                        return true;
                    }
                }
            }
        }
        false
    }

    #[cfg(not(windows))]
    fn is_scoop_install() -> bool {
        false
    }

    pub fn self_update() -> Result<String> {
        let install_method = Self::detect_install_method();

        match install_method {
            InstallMethod::Homebrew => {
                anyhow::bail!(
                    "Installed via Homebrew. Please run: brew update && brew upgrade ani-tui"
                );
            }
            InstallMethod::Scoop => {
                anyhow::bail!("Installed via Scoop. Please run: scoop update ani-tui");
            }
            InstallMethod::Binary => {}
        }

        let bin_path = if cfg!(windows) {
            "ani-tui.exe"
        } else {
            "ani-tui"
        };

        let status = self_update::backends::github::Update::configure()
            .repo_owner(REPO_OWNER)
            .repo_name(REPO_NAME)
            .bin_name("ani-tui")
            .bin_path_in_archive(bin_path)
            .show_download_progress(true)
            .current_version(CURRENT_VERSION)
            .build()
            .context("Failed to configure self-update")?
            .update()
            .context("Failed to perform self-update")?;

        Ok(status.version().to_string())
    }
}

impl Default for UpdateChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let v1 = Version::parse("3.7.4").unwrap();
        let v2 = Version::parse("3.7.5").unwrap();
        assert!(v2 > v1);
    }

    #[test]
    fn test_install_method_default() {
        env::remove_var("HOMEBREW_PREFIX");
        let method = UpdateChecker::detect_install_method();
        assert_eq!(method, InstallMethod::Binary);
    }
}
