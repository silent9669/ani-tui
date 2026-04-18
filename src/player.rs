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
        let mut cmd = Command::new("mpv");

        // Add video URL
        cmd.arg(video_url);

        // Add start time if provided
        if let Some(start) = start_time {
            cmd.arg(format!("--start={}", start));
        }

        // Add headers if provided
        for (key, value) in headers {
            if key.to_lowercase() == "referer" {
                cmd.arg(format!("--referrer={}", value));
            } else {
                cmd.arg(format!("--http-header-name={}: {}", key, value));
            }
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

        // Detach completely from parent process
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0); // Create new process group
        }

        // Spawn and forget - don't wait for it
        let _ = cmd
            .spawn()
            .with_context(|| "Failed to start mpv. Is mpv installed?")?;

        Ok(())
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}
