use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::time::Duration;

pub struct Player;

impl Player {
    pub fn new() -> Self {
        Self
    }

    /// Start player and return immediately (non-blocking)
    pub fn start_detached(
        &self,
        video_url: &str,
        subtitles: &[crate::providers::Subtitle],
        headers: &HashMap<String, String>,
        start_time: Option<u64>,
    ) -> Result<()> {
        let player_command = Self::resolve_player_command()?;

        // Log to file for "Report" feature
        let log_file = std::env::temp_dir().join("ani-tui-mpv.log");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .context("Failed to open mpv log file")?;

        let mut cmd = Self::build_command(
            &player_command,
            video_url,
            subtitles,
            headers,
            start_time,
            Some(&log_file),
        );
        cmd.stdout(Stdio::from(file.try_clone()?));
        cmd.stderr(Stdio::from(file));
        cmd.stdin(Stdio::null());

        // Detach completely from parent process
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0); // Create new process group
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to start {}. Is mpv installed?", player_command))?;

        std::thread::sleep(Duration::from_millis(1500));
        if let Some(status) = child
            .try_wait()
            .context("Failed to check mpv startup status")?
        {
            let log_tail = Self::read_log_tail(&log_file, 40).unwrap_or_default();
            let mut message = format!("mpv exited before playback could start ({})", status);
            if !log_tail.trim().is_empty() {
                message.push_str(&format!("\nRecent mpv log:\n{}", log_tail));
            }
            anyhow::bail!(message);
        }

        Ok(())
    }

    fn build_command(
        player_command: &str,
        video_url: &str,
        subtitles: &[crate::providers::Subtitle],
        headers: &HashMap<String, String>,
        start_time: Option<u64>,
        log_file: Option<&std::path::Path>,
    ) -> Command {
        let mut cmd = Command::new(player_command);

        cmd.arg(video_url);

        if let Some(start) = start_time {
            cmd.arg(format!("--start={}", start));
        }

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

        for subtitle in subtitles {
            if !subtitle.url.trim().is_empty() {
                cmd.arg(format!("--sub-file={}", subtitle.url));
            }
        }

        cmd.arg("--force-media-title=ani-tui");
        cmd.arg("--force-window=immediate");
        cmd.arg("--keep-open=no");
        cmd.arg("--msg-level=all=v");
        cmd.arg("--msg-time");
        cmd.arg("--ytdl-format=bestvideo+bestaudio/best");
        cmd.arg("--hls-bitrate=max");

        if let Some(log_file) = log_file {
            cmd.arg(format!("--log-file={}", log_file.display()));
        }

        cmd
    }

    fn read_log_tail(path: &std::path::Path, max_lines: usize) -> Result<String> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read mpv log at {}", path.display()))?;
        let lines: Vec<&str> = content.lines().rev().take(max_lines).collect();
        Ok(lines.into_iter().rev().collect::<Vec<_>>().join("\n"))
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
