use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::{Command, Stdio};

pub struct Player;

impl Player {
    pub fn new() -> Self {
        Self
    }

    /// Start player and return immediately (non-blocking)
    pub fn start_detached(
        &self,
        video_url: &str,
        _subtitles: &[crate::providers::Subtitle],
        headers: &HashMap<String, String>,
        start_time: Option<u64>,
    ) -> Result<()> {
        let player_command = Self::resolve_player_command()?;
        let mut cmd = Command::new(&player_command);

        // Add video URL
        cmd.arg(video_url);

        // Add start time if provided
        if let Some(start) = start_time {
            cmd.arg(format!("--start={}", start));
        }

        // Add headers if provided
        let mut header_fields = Vec::new();
        for (key, value) in headers {
            match key.to_lowercase().as_str() {
                "referer" => {
                    cmd.arg(format!("--referrer={}", value));
                }
                "user-agent" => {
                    cmd.arg(format!("--user-agent={}", value));
                }
                _ => {
                    header_fields.push(format!("{}: {}", key, value));
                }
            }
        }

        if !header_fields.is_empty() {
            cmd.arg(format!("--http-header-fields={}", header_fields.join(",")));
        }

        // Force media title
        cmd.arg("--force-media-title=ani-tui");

        // Don't exit immediately on error
        cmd.arg("--keep-open=no");

        // Log to file for "Report" feature
        let log_file = std::env::temp_dir().join("ani-tui-mpv.log");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .context("Failed to open mpv log file")?;

        cmd.stdout(Stdio::from(file.try_clone()?));
        cmd.stderr(Stdio::from(file));
        cmd.stdin(Stdio::null());

        // Force mpv to flush logs and be more verbose for debugging
        cmd.arg("--msg-level=all=v");
        cmd.arg("--msg-time");

        // Force highest quality streaming
        cmd.arg("--ytdl-format=bestvideo+bestaudio/best");
        cmd.arg("--hls-bitrate=max");

        // Detach completely from parent process
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0); // Create new process group
        }

        // Spawn and forget - don't wait for it
        let _ = cmd
            .spawn()
            .with_context(|| format!("Failed to start {}. Is mpv installed?", player_command))?;

        Ok(())
    }

    fn resolve_player_command() -> Result<String> {
        let env_player = std::env::var("ANI_TUI_PLAYER").ok();
        Self::resolve_player_command_with(env_player.as_deref(), Self::command_exists)
    }

    fn resolve_player_command_with(
        env_player: Option<&str>,
        command_exists: impl Fn(&str) -> bool,
    ) -> Result<String> {
        if let Some(command) = env_player
            .map(str::trim)
            .filter(|command| !command.is_empty())
        {
            return Ok(command.to_string());
        }
        for command in ["mpv", "mpv.exe"] {
            if command_exists(command) {
                return Ok(command.to_string());
            }
        }

        #[cfg(windows)]
        anyhow::bail!(
            "mpv was not found. Install it with `winget install mpv` or set ANI_TUI_PLAYER to the full mpv.exe path."
        );

        #[cfg(not(windows))]
        anyhow::bail!("mpv was not found. Install mpv or set ANI_TUI_PLAYER to the player path.");
    }

    fn command_exists(command: &str) -> bool {
        Command::new(command)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_player_prefers_env_override() {
        let command = Player::resolve_player_command_with(Some("C:\\tools\\mpv.exe"), |_| false)
            .expect("env override should be accepted");

        assert_eq!(command, "C:\\tools\\mpv.exe");
    }

    #[test]
    fn resolve_player_falls_back_to_mpv_exe() {
        let command = Player::resolve_player_command_with(None, |candidate| candidate == "mpv.exe")
            .expect("mpv.exe fallback should be accepted");

        assert_eq!(command, "mpv.exe");
    }

    #[test]
    fn resolve_player_errors_when_missing() {
        let error = Player::resolve_player_command_with(None, |_| false)
            .expect_err("missing player should error")
            .to_string();

        assert!(error.contains("mpv was not found"));
    }
}
