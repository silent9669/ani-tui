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
            }
        }

        // Force media title
        cmd.arg("--force-media-title=ani-tui");

        // Don't exit immediately on error
        cmd.arg("--keep-open=no");

        // Hide mpv console output
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        cmd.stdin(Stdio::null());

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
