use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};
use std::io::Write;
use std::process::{Command, Stdio};

/// Supported image rendering protocols
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Protocol {
    /// Kitty graphics protocol (best quality - Warp, Kitty, Ghostty, WezTerm)
    Kitty,
    /// iTerm2 inline images (macOS)
    Iterm2,
    /// Sixel graphics (Windows Terminal via chafa)
    Sixel,
    /// No image support (Terminal.app)
    None,
}

impl Protocol {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Protocol::Kitty => "Kitty Graphics Protocol",
            Protocol::Iterm2 => "iTerm2 Inline Images",
            Protocol::Sixel => "Sixel Graphics",
            Protocol::None => "None",
        }
    }
}

/// Errors that can occur during image rendering
#[derive(Debug)]
pub enum ImageError {
    ProtocolNotSupported(String),
    ChafaNotInstalled,
    InvalidImageData(String),
    RenderFailed(String),
    TerminalTooSmall,
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageError::ProtocolNotSupported(term) => {
                write!(f, "Images not supported in {}", term)
            }
            ImageError::ChafaNotInstalled => {
                write!(f, "chafa not installed")
            }
            ImageError::InvalidImageData(msg) => {
                write!(f, "Invalid image data: {}", msg)
            }
            ImageError::RenderFailed(msg) => {
                write!(f, "Failed to render image: {}", msg)
            }
            ImageError::TerminalTooSmall => {
                write!(f, "Terminal too small for image")
            }
        }
    }
}

impl std::error::Error for ImageError {}

impl ImageError {
    /// Convert error to displayable lines
    pub fn to_lines(&self) -> Vec<Line<'static>> {
        match self {
            ImageError::ProtocolNotSupported(term) => {
                vec![
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "⚠️  Images not supported",
                        Style::default().fg(Color::Yellow),
                    )]),
                    Line::from(""),
                    Line::from(format!("Terminal: {}", term)),
                    Line::from(""),
                    Line::from("This terminal cannot display images."),
                    Line::from(""),
                    Line::from("Recommended alternatives:"),
                    Line::from(vec![
                        Span::raw("• macOS: "),
                        Span::styled("Warp", Style::default().fg(Color::Green)),
                    ]),
                    Line::from(vec![
                        Span::raw("• Windows: "),
                        Span::styled("Windows Terminal", Style::default().fg(Color::Green)),
                    ]),
                ]
            }
            ImageError::ChafaNotInstalled => {
                vec![
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "⚠️  chafa not found",
                        Style::default().fg(Color::Yellow),
                    )]),
                    Line::from(""),
                    Line::from("Install chafa for image previews:"),
                    Line::from(""),
                    #[cfg(target_os = "windows")]
                    Line::from(vec![Span::styled(
                        "  scoop install chafa",
                        Style::default().fg(Color::Cyan),
                    )]),
                    #[cfg(target_os = "macos")]
                    Line::from(vec![Span::styled(
                        "  brew install chafa",
                        Style::default().fg(Color::Cyan),
                    )]),
                    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
                    Line::from(vec![Span::styled(
                        "  Install chafa via your package manager",
                        Style::default().fg(Color::Cyan),
                    )]),
                ]
            }
            ImageError::InvalidImageData(msg) => {
                vec![
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "⚠️  Invalid image data",
                        Style::default().fg(Color::Red),
                    )]),
                    Line::from(format!("Error: {}", msg)),
                ]
            }
            ImageError::RenderFailed(msg) => {
                vec![
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "⚠️  Failed to render image",
                        Style::default().fg(Color::Red),
                    )]),
                    Line::from(format!("Error: {}", msg)),
                ]
            }
            ImageError::TerminalTooSmall => {
                vec![
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "⚠️  Terminal too small",
                        Style::default().fg(Color::Yellow),
                    )]),
                    Line::from("Resize terminal to see image preview"),
                ]
            }
        }
    }
}

/// Image renderer that auto-detects terminal capabilities
pub struct ImageRenderer {
    protocol: Protocol,
    sixel_cache: Option<String>,
    kitty_image_id: u32,
}

impl ImageRenderer {
    /// Create a new image renderer with auto-detected protocol
    pub fn new() -> Self {
        let protocol = Self::detect_protocol();
        tracing::info!("Image renderer initialized with protocol: {:?}", protocol);

        Self {
            protocol,
            sixel_cache: None,
            kitty_image_id: 1,
        }
    }

    /// Get the detected protocol
    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    /// Auto-detect the best available protocol
    ///
    /// Note: We prefer Sixel when available because it renders as text lines
    /// that integrate well with ratatui's frame-based rendering.
    /// Kitty/iTerm2 protocols output escape codes directly to stdout which
    /// can disrupt the TUI layout.
    fn detect_protocol() -> Protocol {
        let term = std::env::var("TERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();

        tracing::debug!(
            "Terminal detection - TERM: {}, TERM_PROGRAM: {}",
            term,
            term_program
        );

        // 1. Check for Terminal.app (no image support)
        if term_program == "Apple_Terminal" {
            tracing::warn!("Detected Terminal.app - no image support");
            return Protocol::None;
        }

        // 2. Prefer Sixel via chafa when available (best integration with ratatui)
        // This works on Windows Terminal, most Linux terminals, and many modern terminals
        if Self::is_chafa_available() {
            // Check if this is a terminal known to support Sixel well
            if term_program == "WarpTerminal"
                || term_program == "warp"
                || term == "xterm-kitty"
                || std::env::var("KITTY_WINDOW_ID").is_ok()
                || term_program == "WezTerm"
                || term_program == "ghostty"
                || term_program == "Ghostty"
                || cfg!(target_os = "windows")
                || std::env::var("WT_SESSION").is_ok()
            {
                tracing::info!("Detected terminal with Sixel support via chafa");
                return Protocol::Sixel;
            }

            // Default to Sixel for any terminal when chafa is available
            // (Sixel is widely supported in modern terminals)
            tracing::info!("Using Sixel protocol via chafa");
            return Protocol::Sixel;
        }

        // 3. Fallback to Kitty protocol for Kitty terminal (if chafa not available)
        if term == "xterm-kitty" || std::env::var("KITTY_WINDOW_ID").is_ok() {
            tracing::info!("Detected Kitty terminal (chafa not available)");
            return Protocol::Kitty;
        }

        // 4. Fallback to iTerm2 for iTerm (if chafa not available)
        if term_program == "iTerm.app" {
            tracing::info!("Detected iTerm2 (chafa not available)");
            return Protocol::Iterm2;
        }

        tracing::warn!("No image protocol available (install chafa for best compatibility)");
        Protocol::None
    }

    /// Check if chafa is installed
    fn is_chafa_available() -> bool {
        Command::new("chafa")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Render an image to the terminal
    ///
    /// Returns Ok(None) if image was rendered via escape codes (Kitty, iTerm2)
    /// Returns Ok(Some(lines)) for Sixel or error display
    pub fn render(
        &mut self,
        image_data: &[u8],
        area: Rect,
    ) -> Result<Option<Vec<Line<'static>>>, ImageError> {
        // Validate image data
        if image_data.is_empty() {
            return Err(ImageError::InvalidImageData("Empty image data".to_string()));
        }

        // Validate image format
        if !self.is_valid_image(image_data) {
            return Err(ImageError::InvalidImageData(
                "Invalid image format".to_string(),
            ));
        }

        // Check terminal size
        if area.width < 10 || area.height < 5 {
            return Err(ImageError::TerminalTooSmall);
        }

        match self.protocol {
            Protocol::Kitty => {
                self.render_kitty(image_data, area)?;
                Ok(None) // Image rendered via escape codes
            }
            Protocol::Iterm2 => {
                self.render_iterm2(image_data, area)?;
                Ok(None) // Image rendered via escape codes
            }
            Protocol::Sixel => {
                let lines = self.render_sixel(image_data, area)?;
                Ok(Some(lines))
            }
            Protocol::None => {
                let term = std::env::var("TERM_PROGRAM").unwrap_or_else(|_| "Unknown".to_string());
                Err(ImageError::ProtocolNotSupported(term))
            }
        }
    }

    /// Validate image format by checking magic bytes
    fn is_valid_image(&self, data: &[u8]) -> bool {
        if data.len() < 8 {
            return false;
        }

        // Check for valid image formats
        let is_png = data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        let is_jpeg = data.starts_with(&[0xFF, 0xD8, 0xFF]);
        let is_webp = data.starts_with(&[0x52, 0x49, 0x46, 0x46]);
        let is_gif = data.starts_with(&[0x47, 0x49, 0x46, 0x38]);
        let is_bmp = data.starts_with(&[0x42, 0x4D]);

        is_png || is_jpeg || is_webp || is_gif || is_bmp
    }

    /// Render using Kitty graphics protocol
    ///
    /// Format: ESC _G<control_data>;<payload>ESC \
    fn render_kitty(&mut self, image_data: &[u8], area: Rect) -> Result<(), ImageError> {
        let image_id = self.kitty_image_id;
        self.kitty_image_id = self.kitty_image_id.wrapping_add(1);

        // Get image dimensions
        let (img_width, img_height) = self.get_image_dimensions(image_data)?;

        // Calculate display size to fit within area while maintaining aspect ratio
        let (display_cols, display_rows) = self.calculate_display_size(
            img_width,
            img_height,
            area.width as u32,
            area.height as u32,
        );

        // For Kitty protocol, we need to output escape codes directly to stdout
        // We'll use PNG format (f=100) for best compatibility

        let stdout = std::io::stdout();
        let mut handle = stdout.lock();

        // First, delete any existing image with this ID to avoid conflicts
        let delete_cmd = format!("\x1b_Ga=d,d=i,i={}\x1b\\", image_id);
        handle
            .write_all(delete_cmd.as_bytes())
            .map_err(|e| ImageError::RenderFailed(e.to_string()))?;

        // Send image data in chunks
        // Format: ESC _Gf=100,i=<id>,s=<w>,v=<h>,a=T;<base64_data>ESC \

        let base64_data = general_purpose::STANDARD.encode(image_data);
        let chunk_size = 4096;
        let chunks: Vec<&str> = base64_data
            .as_bytes()
            .chunks(chunk_size)
            .map(|chunk| std::str::from_utf8(chunk).unwrap())
            .collect();

        for (i, chunk) in chunks.iter().enumerate() {
            let is_first = i == 0;
            let is_last = i == chunks.len() - 1;

            let control = if is_first {
                format!(
                    "f=100,i={},s={},v={},a=T,m={}",
                    image_id,
                    img_width,
                    img_height,
                    if is_last { 0 } else { 1 }
                )
            } else {
                format!("m={}", if is_last { 0 } else { 1 })
            };

            let cmd = format!("\x1b_G{};{}\x1b\\", control, chunk);
            handle
                .write_all(cmd.as_bytes())
                .map_err(|e| ImageError::RenderFailed(e.to_string()))?;
        }

        // Create placement at cursor position
        // ESC _Ga=p,i=<id>,c=<cols>,r=<rows>ESC \
        let place_cmd = format!(
            "\x1b_Ga=p,i={},c={},r={}\x1b\\",
            image_id, display_cols, display_rows
        );
        handle
            .write_all(place_cmd.as_bytes())
            .map_err(|e| ImageError::RenderFailed(e.to_string()))?;

        handle
            .flush()
            .map_err(|e| ImageError::RenderFailed(e.to_string()))?;

        tracing::debug!(
            "Rendered image {} via Kitty protocol ({}x{} cells)",
            image_id,
            display_cols,
            display_rows
        );

        Ok(())
    }

    /// Render using iTerm2 inline images protocol
    ///
    /// Format: ESC ] 1337 ; File=<params>:<base64_data> BEL
    fn render_iterm2(&self, image_data: &[u8], area: Rect) -> Result<(), ImageError> {
        // Get image dimensions
        let (img_width, img_height) = self.get_image_dimensions(image_data)?;

        // Calculate display size
        let (display_cols, display_rows) = self.calculate_display_size(
            img_width,
            img_height,
            area.width as u32,
            area.height as u32,
        );

        let base64_data = general_purpose::STANDARD.encode(image_data);

        // iTerm2 OSC 1337 format
        // ESC ] 1337 ; File=inline=1,width=<w>,height=<h>:<base64> BEL
        let osc = format!(
            "\x1b]1337;File=inline=1,width={},height={},preserveAspectRatio=1:{}\x07",
            display_cols, display_rows, base64_data
        );

        let stdout = std::io::stdout();
        let mut handle = stdout.lock();

        handle
            .write_all(osc.as_bytes())
            .map_err(|e| ImageError::RenderFailed(e.to_string()))?;
        handle
            .flush()
            .map_err(|e| ImageError::RenderFailed(e.to_string()))?;

        tracing::debug!(
            "Rendered image via iTerm2 protocol ({}x{} cells)",
            display_cols,
            display_rows
        );

        Ok(())
    }

    /// Render using Sixel via chafa
    fn render_sixel(
        &mut self,
        image_data: &[u8],
        area: Rect,
    ) -> Result<Vec<Line<'static>>, ImageError> {
        // Check cache
        if let Some(ref cached) = self.sixel_cache {
            return Ok(self.parse_sixel_to_lines(cached, area));
        }

        // Spawn chafa process
        let mut child = Command::new("chafa")
            .args(&[
                "-f",
                "sixels",
                "--size",
                &format!("{}x{}", area.width, area.height * 2), // *2 for better resolution
                "--center",
                "on",
                "--colors",
                "256",
                "--dither",
                "ordered", // Better quality
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| ImageError::ChafaNotInstalled)?;

        // Write image data
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(image_data).map_err(|e| {
                ImageError::RenderFailed(format!("Failed to write to chafa: {}", e))
            })?;
        }

        // Wait for output
        let output = child
            .wait_with_output()
            .map_err(|e| ImageError::RenderFailed(format!("chafa failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ImageError::RenderFailed(format!("chafa error: {}", stderr)));
        }

        let sixel_output = String::from_utf8(output.stdout)
            .map_err(|e| ImageError::RenderFailed(format!("Invalid UTF-8 from chafa: {}", e)))?;

        // Cache for reuse
        self.sixel_cache = Some(sixel_output.clone());

        Ok(self.parse_sixel_to_lines(&sixel_output, area))
    }

    /// Parse Sixel output to ratatui Lines
    fn parse_sixel_to_lines(&self, sixel: &str, area: Rect) -> Vec<Line<'static>> {
        let lines: Vec<&str> = sixel.lines().collect();
        let max_lines = area.height as usize;
        let max_width = area.width as usize;

        lines
            .iter()
            .take(max_lines)
            .map(|line| {
                let truncated = if line.len() > max_width {
                    &line[..max_width]
                } else {
                    line
                };
                Line::from(Span::raw(truncated.to_string()))
            })
            .collect()
    }

    /// Get image dimensions
    fn get_image_dimensions(&self, image_data: &[u8]) -> Result<(u32, u32), ImageError> {
        // Use image crate to get dimensions
        match image::load_from_memory(image_data) {
            Ok(img) => Ok((img.width(), img.height())),
            Err(e) => Err(ImageError::InvalidImageData(format!(
                "Cannot load image: {}",
                e
            ))),
        }
    }

    /// Calculate display size maintaining aspect ratio
    fn calculate_display_size(
        &self,
        img_width: u32,
        img_height: u32,
        max_cols: u32,
        max_rows: u32,
    ) -> (u32, u32) {
        let aspect_ratio = img_width as f32 / img_height as f32;

        // Try to fit in max dimensions
        let mut cols = max_cols;
        let mut rows = (cols as f32 / aspect_ratio) as u32;

        // If too tall, scale down
        if rows > max_rows {
            rows = max_rows;
            cols = (rows as f32 * aspect_ratio) as u32;
        }

        // Ensure minimum size
        cols = cols.max(10);
        rows = rows.max(5);

        (cols, rows)
    }

    /// Clear cache (call when switching images)
    pub fn clear_cache(&mut self) {
        self.sixel_cache = None;
    }
}

impl Default for ImageRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_detection() {
        // This would need to set env vars, so we just test the struct creation
        let renderer = ImageRenderer::new();
        // Should not panic
        let _ = renderer.protocol();
    }

    #[test]
    fn test_error_display() {
        let err = ImageError::ChafaNotInstalled;
        let lines = err.to_lines();
        assert!(!lines.is_empty());
    }
}
