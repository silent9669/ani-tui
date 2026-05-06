use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use crossterm::cursor::MoveTo;
use crossterm::queue;
use image::{imageops::FilterType, GenericImageView};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::time::Instant;

const MIN_RENDER_INTERVAL_MS: u128 = 33;
const TERMINAL_CELL_WIDTH_TO_HEIGHT: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Protocol {
    /// Kitty graphics protocol (best quality - Warp, Kitty, Ghostty, WezTerm)
    Kitty,
    /// iTerm2 inline images (macOS)
    Iterm2,
    /// Sixel graphics (via chafa - works on most modern terminals)
    Sixel,
    /// Halfblock terminal rendering (cross-platform, fallback)
    Halfblocks,
    /// No image support (Terminal.app)
    None,
}

impl Protocol {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Protocol::Kitty => "Kitty Graphics Protocol",
            Protocol::Iterm2 => "iTerm2 Inline Images",
            Protocol::Sixel => "Sixel Graphics (chafa)",
            Protocol::Halfblocks => "Halfblocks (Native fallback)",
            Protocol::None => "None",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ProtocolDetectionContext<'a> {
    term: &'a str,
    term_program: &'a str,
    warp_session: bool,
    kitty_window_id: bool,
    wt_session: bool,
    wt_version: Option<&'a str>,
    forced_protocol: Option<&'a str>,
    chafa_available: bool,
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
                write!(f, "chafa not installed (scoop install chafa)")
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
                        "⚠️ Images not supported",
                        Style::default().fg(Color::Yellow),
                    )]),
                    Line::from(""),
                    Line::from(format!("Terminal: {}", term)),
                    Line::from(""),
                    Line::from("This terminal cannot display images."),
                    Line::from(""),
                    Line::from("Recommended terminals:"),
                    Line::from(vec![
                        Span::raw("  macOS: "),
                        Span::styled("Warp, iTerm2, Kitty", Style::default().fg(Color::Green)),
                    ]),
                    Line::from(vec![
                        Span::raw("  Windows: "),
                        Span::styled("Windows Terminal", Style::default().fg(Color::Green)),
                    ]),
                    Line::from(vec![
                        Span::raw("  Linux: "),
                        Span::styled(
                            "Kitty, WezTerm, GNOME Terminal",
                            Style::default().fg(Color::Green),
                        ),
                    ]),
                ]
            }
            ImageError::ChafaNotInstalled => {
                vec![
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "⚠️ chafa not found",
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
                        "⚠️ Invalid image data",
                        Style::default().fg(Color::Red),
                    )]),
                    Line::from(format!("Error: {}", msg)),
                ]
            }
            ImageError::RenderFailed(msg) => {
                vec![
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "⚠️ Failed to render image",
                        Style::default().fg(Color::Red),
                    )]),
                    Line::from(format!("Error: {}", msg)),
                ]
            }
            ImageError::TerminalTooSmall => {
                vec![
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "⚠️ Terminal too small",
                        Style::default().fg(Color::Yellow),
                    )]),
                    Line::from("Resize terminal to see image preview"),
                ]
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum RenderOutput {
    Escape(String),
    Halfblocks(Vec<Line<'static>>),
}

use ratatui::widgets::Widget;

pub struct TerminalImage {
    output: Option<RenderOutput>,
}

impl TerminalImage {
    pub fn new(output: Option<RenderOutput>) -> Self {
        Self { output }
    }
}

impl Widget for TerminalImage {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        if let Some(RenderOutput::Halfblocks(lines)) = self.output {
            for (y, line) in (area.y..).zip(lines) {
                if y >= area.y + area.height {
                    break;
                }
                buf.set_line(area.x, y, &line, area.width);
            }
        }
    }
}

pub struct ImageRenderer {
    protocol: Protocol,
    sixel_cache: Option<Vec<u8>>,
    last_kitty_image_id: Option<u32>,
    last_rendered_hash: Option<u64>,
    last_rendered_area: Option<Rect>,
    last_image_data: Option<Vec<u8>>,
    last_render_time: Instant,
    last_halfblocks_lines: Option<Vec<Line<'static>>>,
    last_sequences: Option<String>,
}

impl ImageRenderer {
    /// Create a new image renderer with auto-detected protocol
    pub fn new() -> Self {
        let protocol = Self::detect_protocol();
        tracing::info!("Image renderer initialized with protocol: {:?}", protocol);

        Self {
            protocol,
            sixel_cache: None,
            last_kitty_image_id: None,
            last_rendered_hash: None,
            last_rendered_area: None,
            last_image_data: None,
            last_render_time: Instant::now(),
            last_halfblocks_lines: None,
            last_sequences: None,
        }
    }

    pub fn render_to_widget(&mut self, image_data: &[u8], area: Rect) -> TerminalImage {
        match self.render(image_data, area) {
            Ok(Some(output)) => {
                if let RenderOutput::Escape(ref seq) = output {
                    self.last_sequences = Some(seq.clone());
                }
                TerminalImage::new(Some(output))
            }
            _ => {
                // If skipped or error, try to return last halfblocks if using that protocol
                if matches!(self.protocol, Protocol::Halfblocks | Protocol::None) {
                    if let Some(lines) = &self.last_halfblocks_lines {
                        return TerminalImage::new(Some(RenderOutput::Halfblocks(lines.clone())));
                    }
                }
                TerminalImage::new(None)
            }
        }
    }

    pub fn flush_sequences(&mut self) -> std::io::Result<()> {
        use std::io::Write;
        if let Some(seq) = self.last_sequences.take() {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            handle.write_all(seq.as_bytes())?;
            handle.flush()?;
        }
        Ok(())
    }

    /// Get the detected protocol
    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    /// Auto-detect the best available protocol
    ///
    /// Protocol priority based on terminal compatibility research:
    /// - iTerm2 protocol: Widely supported (iTerm2, Warp, WezTerm, VSCode)
    /// - Kitty protocol: Best quality but limited support (Kitty, Ghostty)
    /// - Sixel: Good fallback via chafa (Windows Terminal, foot, etc.)
    fn detect_protocol() -> Protocol {
        let term = std::env::var("TERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        let warp_session = std::env::var("WARP_SESSION_ID").is_ok();
        let kitty_window_id = std::env::var("KITTY_WINDOW_ID").is_ok();
        let wt_session = std::env::var("WT_SESSION").is_ok();
        let wt_version = std::env::var("WT_VERSION").ok();
        let forced_protocol = std::env::var("ANI_TUI_IMAGE_PROTOCOL").ok();

        tracing::debug!(
            "Terminal detection - TERM: {}, TERM_PROGRAM: {}, WARP_SESSION: {}, KITTY_WINDOW_ID: {}, WT_SESSION: {}",
            term,
            term_program,
            warp_session,
            kitty_window_id,
            wt_session
        );

        Self::detect_protocol_from_env(ProtocolDetectionContext {
            term: &term,
            term_program: &term_program,
            warp_session,
            kitty_window_id,
            wt_session,
            wt_version: wt_version.as_deref(),
            forced_protocol: forced_protocol.as_deref(),
            chafa_available: Self::is_chafa_available(),
        })
    }

    fn detect_protocol_from_env(ctx: ProtocolDetectionContext<'_>) -> Protocol {
        let ProtocolDetectionContext {
            term,
            term_program,
            warp_session,
            kitty_window_id,
            wt_session,
            wt_version,
            forced_protocol,
            chafa_available,
        } = ctx;

        if let Some(protocol) = forced_protocol {
            match protocol.trim().to_ascii_lowercase().as_str() {
                "auto" | "" => {}
                "kitty" => {
                    tracing::info!("Image protocol forced to Kitty");
                    return Protocol::Kitty;
                }
                "iterm" | "iterm2" => {
                    tracing::info!("Image protocol forced to iTerm2");
                    return Protocol::Iterm2;
                }
                "sixel" | "sixels" => {
                    if chafa_available {
                        tracing::info!("Image protocol forced to Sixel");
                        return Protocol::Sixel;
                    }
                    tracing::warn!("Sixel protocol requested but chafa is not available");
                    return Protocol::Halfblocks;
                }
                "halfblock" | "halfblocks" | "native" => {
                    tracing::info!("Image protocol forced to Halfblocks");
                    return Protocol::Halfblocks;
                }
                unknown => {
                    tracing::warn!(
                        "Unknown ANI_TUI_IMAGE_PROTOCOL value '{}', using auto detection",
                        unknown
                    );
                }
            }
        }

        // Native truecolor halfblocks are the most stable cross-platform solution
        // avoiding "image protocol problem of the terminal on both window and mac"
        // so we default to it when high-fidelity protocols aren't strictly detected

        // 1. Terminal.app - no image support natively, use Halfblocks
        if term_program == "Apple_Terminal" {
            tracing::info!("Detected Terminal.app - using Halfblocks fallback");
            return Protocol::Halfblocks;
        }

        // 2. iTerm2 - native iTerm2 protocol support
        if term_program == "iTerm.app" {
            tracing::info!("Detected iTerm2 - using iTerm2 protocol");
            return Protocol::Iterm2;
        }

        // 3. Warp - use iTerm2 protocol for better compatibility
        if term_program == "WarpTerminal" || warp_session {
            tracing::info!("Detected Warp terminal - using iTerm2 protocol");
            return Protocol::Iterm2;
        }

        // 4. WezTerm - supports both Kitty and iTerm2, but Kitty is more robust
        if term_program == "WezTerm" {
            tracing::info!("Detected WezTerm - using Kitty protocol");
            return Protocol::Kitty;
        }

        // 5. Kitty terminal - native Kitty protocol
        if term == "xterm-kitty" || kitty_window_id {
            tracing::info!("Detected Kitty terminal - using Kitty protocol");
            return Protocol::Kitty;
        }

        // 6. Ghostty - supports Kitty protocol
        if term_program == "Ghostty" || term_program == "ghostty" {
            tracing::info!("Detected Ghostty - using Kitty protocol");
            return Protocol::Kitty;
        }

        // 7. Rio - supports Kitty protocol
        if term_program == "Rio" || term_program == "rio" {
            tracing::info!("Detected Rio terminal - using Kitty protocol");
            return Protocol::Kitty;
        }

        // 8. Windows Terminal - Check for iTerm2 support or fallback to Halfblocks
        if wt_session {
            if let Some(version) = wt_version {
                if Self::is_windows_terminal_modern(version) {
                    tracing::info!(
                        "Detected Windows Terminal {} - using iTerm2 protocol",
                        version
                    );
                    return Protocol::Iterm2;
                }
            }

            tracing::info!("Detected Windows Terminal - using Halfblocks fallback");
            return Protocol::Halfblocks;
        }

        // 9. Sixel is only safe when the terminal is explicitly known to support it.
        let term_lower = term.to_ascii_lowercase();
        if chafa_available
            && (term_lower.contains("foot")
                || term_lower.contains("mlterm")
                || term_lower.contains("contour"))
        {
            tracing::info!("Detected sixel-capable terminal - using Sixel via chafa");
            return Protocol::Sixel;
        }

        tracing::info!("Using Halfblocks fallback for unknown terminal");
        Protocol::Halfblocks
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

    /// Check if Windows Terminal version supports iTerm2 protocol (v1.22+)
    fn is_windows_terminal_modern(version: &str) -> bool {
        // Parse version like "1.22.10352.0"
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() >= 2 {
            if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                return major > 1 || (major == 1 && minor >= 22);
            }
        }
        false
    }

    /// Render using Halfblocks (native terminal characters)
    fn render_halfblocks(
        &mut self,
        image_data: &[u8],
        area: Rect,
    ) -> Result<Vec<Line<'static>>, ImageError> {
        let img = image::load_from_memory(image_data)
            .map_err(|e| ImageError::InvalidImageData(e.to_string()))?;

        // Calculate size maintaining aspect ratio
        let img_width = img.width() as f32;
        let img_height = img.height() as f32;

        let term_width = area.width as f32;
        // terminal cells are roughly 2x as tall as they are wide.
        // since we use half blocks, 1 cell = 2 vertical pixels.
        let term_height_px = (area.height as f32) * 2.0;

        let width_ratio = term_width / img_width;
        let height_ratio = term_height_px / img_height;
        let scale = width_ratio.min(height_ratio).min(1.0); // Don't upscale

        let target_width = (img_width * scale).max(1.0) as u32;
        let target_height = (img_height * scale).max(2.0) as u32;

        let resized = img.resize_exact(target_width, target_height, FilterType::Nearest);

        let mut lines = Vec::new();
        // Since we are rendering to a specific Rect area, we want to center the image if it's smaller
        let pad_x = (area.width as u32).saturating_sub(target_width) / 2;
        let pad_y_cells = (area.height as u32).saturating_sub(target_height / 2) / 2;

        // Top padding
        for _ in 0..pad_y_cells {
            lines.push(Line::from(""));
        }

        for y in (0..target_height).step_by(2) {
            let mut spans = Vec::new();

            // Left padding
            if pad_x > 0 {
                spans.push(Span::raw(" ".repeat(pad_x as usize)));
            }

            for x in 0..target_width {
                let top = resized.get_pixel(x, y);
                let bottom = if y + 1 < target_height {
                    resized.get_pixel(x, y + 1)
                } else {
                    top // fallback
                };

                let top_color = Color::Rgb(top[0], top[1], top[2]);
                let bottom_color = Color::Rgb(bottom[0], bottom[1], bottom[2]);

                spans.push(Span::styled(
                    "▀",
                    Style::default().fg(top_color).bg(bottom_color),
                ));
            }
            lines.push(Line::from(spans));
        }

        // Bottom padding will be handled naturally by the widget

        Ok(lines)
    }

    /// Render an image to the terminal
    ///
    /// Returns Ok(Some(String)) with escape sequences if using a high-fidelity protocol
    /// Returns Ok(Some(Vec<Line>)) if using Halfblocks
    /// Returns Ok(None) if image was skipped (already rendered)
    /// Returns Err if rendering failed
    pub fn render(
        &mut self,
        image_data: &[u8],
        area: Rect,
    ) -> Result<Option<RenderOutput>, ImageError> {
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

        if area.width < 10 || area.height < 5 {
            return Err(ImageError::TerminalTooSmall);
        }

        let elapsed = self.last_render_time.elapsed().as_millis();
        if elapsed < MIN_RENDER_INTERVAL_MS && self.last_rendered_hash.is_some() {
            tracing::debug!(
                "Skipping render - too soon ({}ms < {}ms)",
                elapsed,
                MIN_RENDER_INTERVAL_MS
            );
            return Ok(None);
        }

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        image_data.hash(&mut hasher);
        let current_hash = hasher.finish();

        let should_render = match self.protocol {
            Protocol::Sixel => true,
            _ => match (&self.last_rendered_hash, &self.last_rendered_area) {
                (Some(last_hash), Some(last_area)) => {
                    *last_hash != current_hash || *last_area != area
                }
                _ => true,
            },
        };

        if !should_render {
            tracing::debug!("Skipping image render - already rendered at this position");
            if matches!(self.protocol, Protocol::Halfblocks | Protocol::None) {
                if let Some(lines) = &self.last_halfblocks_lines {
                    return Ok(Some(RenderOutput::Halfblocks(lines.clone())));
                }
            }
            return Ok(None);
        }

        tracing::debug!("Rendering image (hash: {}, area: {:?})", current_hash, area);

        // Render based on protocol
        let result = match self.protocol {
            Protocol::Kitty => {
                let sequences = self.render_kitty(image_data, area)?;
                Ok(Some(RenderOutput::Escape(sequences)))
            }
            Protocol::Iterm2 => {
                let sequences = self.render_iterm2(image_data, area)?;
                Ok(Some(RenderOutput::Escape(sequences)))
            }
            Protocol::Sixel => {
                let sequences = self.render_sixel(image_data, area)?;
                Ok(Some(RenderOutput::Escape(sequences)))
            }
            Protocol::Halfblocks | Protocol::None => {
                let lines = self.render_halfblocks(image_data, area)?;
                self.last_halfblocks_lines = Some(lines.clone());
                Ok(Some(RenderOutput::Halfblocks(lines)))
            }
        };

        if result.is_ok() {
            self.last_rendered_hash = Some(current_hash);
            self.last_rendered_area = Some(area);
            self.last_image_data = Some(image_data.to_vec());
            self.last_render_time = Instant::now();
            tracing::debug!("Image rendered successfully, state updated");
        }

        result
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

    /// Check if data starts with PNG magic bytes
    fn data_is_png(data: &[u8]) -> bool {
        data.len() >= 8 && data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
    }

    /// Render using Kitty graphics protocol with cursor-relative positioning
    ///
    /// Uses fixed image ID to prevent accumulation and centers image in the allocated area
    fn render_kitty(&mut self, image_data: &[u8], area: Rect) -> Result<String, ImageError> {
        // Use fixed image ID (always 1) to prevent accumulation and stacking
        let image_id = 1u32;

        // Get image dimensions and detect format
        let (img_width, img_height) = self.get_image_dimensions(image_data)?;

        // Detect format for Kitty protocol (f parameter)
        // 100 = PNG, 24 = RGB, 32 = RGBA
        let png_data = if Self::data_is_png(image_data) {
            image_data.to_vec()
        } else {
            // Load and re-encode as PNG
            let img = image::load_from_memory(image_data)
                .map_err(|e| ImageError::InvalidImageData(e.to_string()))?;
            let mut cursor = std::io::Cursor::new(Vec::new());
            img.write_to(&mut cursor, image::ImageFormat::Png)
                .map_err(|e| ImageError::RenderFailed(format!("Failed to encode as PNG: {}", e)))?;
            cursor.into_inner()
        };

        // Calculate display size to fit within area while maintaining aspect ratio
        let (display_cols, display_rows) = self.calculate_display_size(
            img_width,
            img_height,
            area.width as u32,
            area.height as u32,
        );

        let mut sequences = String::new();

        // Clear ALL previous images to prevent stacking/duplication
        // q=2: suppress all responses to prevent breaking keyboard input
        sequences.push_str("\x1b_Ga=d,d=A,q=2\x1b\\");

        // Send PNG data in chunks without displaying it yet. The explicit
        // placement below is responsible for drawing at the calculated position.
        let base64_data = general_purpose::STANDARD.encode(&png_data);
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
                    "a=t,t=d,f=100,i={},m={},q=2",
                    image_id,
                    if is_last { 0 } else { 1 }
                )
            } else {
                format!("m={}", if is_last { 0 } else { 1 })
            };

            sequences.push_str(&format!("\x1b_G{};{}\x1b\\", control, chunk));
        }

        // Calculate center position within the area
        let start_x = area.x + (area.width.saturating_sub(display_cols as u16)) / 2;
        let start_y = area.y + (area.height.saturating_sub(display_rows as u16)) / 2;

        // Position cursor at the calculated center position
        // We use ANSI escape code directly
        sequences.push_str(&format!("\x1b[{};{}H", start_y + 1, start_x + 1));

        // Create placement at cursor position
        // C=1: do not move cursor after placement
        sequences.push_str(&format!(
            "\x1b_Ga=p,i={},c={},r={},C=1,q=2\x1b\\",
            image_id, display_cols, display_rows
        ));

        // Update state tracking
        self.last_kitty_image_id = Some(image_id);

        Ok(sequences)
    }

    fn render_iterm2(&mut self, image_data: &[u8], area: Rect) -> Result<String, ImageError> {
        const MARGIN: u16 = 3;
        const SIZE_INCREASE: f32 = 2.5;

        let (img_width, img_height) = self.get_image_dimensions(image_data)?;
        let available_width = area.width.saturating_sub(MARGIN * 2);
        let available_height = area.height.saturating_sub(MARGIN * 2);

        let (base_cols, base_rows) = self.calculate_display_size(
            img_width,
            img_height,
            available_width as u32,
            available_height as u32,
        );

        let display_cols = ((base_cols as f32) * SIZE_INCREASE) as u32;
        let display_rows = ((base_rows as f32) * SIZE_INCREASE) as u32;
        let display_cols = display_cols.min(available_width as u32);
        let display_rows = display_rows.min(available_height as u32);

        let start_x = area.x + MARGIN + (available_width.saturating_sub(display_cols as u16)) / 2;
        let start_y = area.y + MARGIN + (available_height.saturating_sub(display_rows as u16)) / 2;

        let base64_data = general_purpose::STANDARD.encode(image_data);

        let mut sequences = String::new();

        // Clear area by writing spaces
        for row in area.y..area.y + area.height {
            sequences.push_str(&format!("\x1b[{};{}H", row + 1, area.x + 1));
            sequences.push_str(&" ".repeat(area.width as usize));
        }

        // Move to start and write image OSC
        sequences.push_str(&format!("\x1b[{};{}H", start_y + 1, start_x + 1));
        sequences.push_str(&format!(
            "\x1b]1337;File=inline=1;size={};width={};height={};preserveAspectRatio=1;doNotMoveCursor=1:{}\x07",
            image_data.len(),
            display_cols,
            display_rows,
            base64_data
        ));

        Ok(sequences)
    }

    /// Render using Sixel via chafa
    fn render_sixel(&mut self, image_data: &[u8], area: Rect) -> Result<String, ImageError> {
        // Check cache - but invalidate if area size changed
        if let Some(ref cached) = self.sixel_cache {
            // Check if cached sixel is for the same area size
            if let Some(last_area) = self.last_rendered_area {
                if last_area.width == area.width && last_area.height == area.height {
                    let mut sequences = String::new();
                    sequences.push_str(&format!("\x1b[{};{}H", area.y + 1, area.x + 1));
                    sequences.push_str(std::str::from_utf8(cached).unwrap_or(""));
                    return Ok(sequences);
                }
            }
            // Area size changed, invalidate cache
            self.sixel_cache = None;
        }

        // Calculate size accounting for borders (2 cells on each side)
        let render_width = area.width.saturating_sub(2);
        let render_height = area.height.saturating_sub(2);

        // Spawn chafa process with high-quality settings
        let mut child = Command::new("chafa")
            .args([
                "-f",
                "sixels",
                "--size",
                &format!("{}x{}", render_width, render_height),
                "--center",
                "on",
                "--colors",
                "256",
                "--symbols",
                "all",
                "--dither",
                "ordered",
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

        // Cache for reuse
        self.sixel_cache = Some(output.stdout.clone());

        let mut sequences = String::new();
        sequences.push_str(&format!("\x1b[{};{}H", area.y + 1, area.x + 1));
        sequences.push_str(std::str::from_utf8(&output.stdout).unwrap_or(""));

        Ok(sequences)
    }

    /// Get image dimensions
    fn get_image_dimensions(&self, image_data: &[u8]) -> Result<(u32, u32), ImageError> {
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
        let pixel_aspect_ratio = img_width as f32 / img_height as f32;
        let cell_aspect_ratio = pixel_aspect_ratio / TERMINAL_CELL_WIDTH_TO_HEIGHT;

        // Use 100% of available space - maximize image size
        let available_cols = max_cols;
        let available_rows = max_rows;

        // Try to fit in max dimensions
        let mut cols = available_cols;
        let mut rows = (cols as f32 / cell_aspect_ratio).round() as u32;

        // If too tall, scale down
        if rows > available_rows {
            rows = available_rows;
            cols = (rows as f32 * cell_aspect_ratio).round() as u32;
        }

        // Ensure minimum size but allow larger images
        cols = cols.clamp(10, available_cols);
        rows = rows.clamp(5, available_rows);

        (cols, rows)
    }

    /// Clear cache (call when switching images)
    pub fn clear_cache(&mut self) {
        self.sixel_cache = None;
        self.last_kitty_image_id = None;
        self.last_rendered_hash = None;
        self.last_rendered_area = None;
        self.last_image_data = None;
        self.last_render_time = Instant::now();
        self.last_halfblocks_lines = None;
        tracing::debug!("Image renderer cache cleared");
    }

    /// Clear graphics from terminal screen using protocol-specific escape sequences
    /// This actually erases images from the terminal, not just from our cache
    pub fn clear_terminal_graphics(&self) -> io::Result<()> {
        let stdout = io::stdout();
        let mut handle = stdout.lock();

        match self.protocol {
            Protocol::Kitty => {
                // Kitty: Delete all images with id=1 (our fixed image ID)
                // a=d: delete action
                // d=i: delete by image id
                // i=1: image id 1
                let clear_cmd = "\x1b_Ga=d,d=i,i=1,q=2\x1b\\";
                handle.write_all(clear_cmd.as_bytes())?;

                // Also send clear all as backup
                let clear_all = "\x1b_Ga=d,d=A,q=2\x1b\\";
                handle.write_all(clear_all.as_bytes())?;

                handle.flush()?;
                tracing::debug!("Terminal graphics cleared for Kitty protocol");
            }
            Protocol::Iterm2 => {
                // iTerm2/Warp: Clear by writing spaces over the last rendered area
                // This is necessary because iTerm2 images persist via escape codes
                if let Some(area) = self.last_rendered_area {
                    for row in area.y..area.y + area.height {
                        queue!(handle, MoveTo(area.x, row)).map_err(|e| {
                            io::Error::other(format!("Failed to position cursor: {}", e))
                        })?;
                        handle.write_all(&vec![b' '; area.width as usize])?;
                    }
                    handle.flush()?;
                    tracing::debug!(
                        "Terminal graphics cleared for iTerm2 protocol (area: {:?})",
                        area
                    );
                } else {
                    tracing::debug!("iTerm2 protocol: no last_rendered_area to clear");
                }
            }
            Protocol::Sixel => {
                // Sixel: Send DCS sequence to end sixel mode and clear
                handle.write_all(b"\x1b\\")?; // String terminator
                handle.flush()?;
                tracing::debug!("Terminal graphics cleared for Sixel protocol");
            }
            Protocol::Halfblocks | Protocol::None => {
                // No graphics to clear
                tracing::debug!("No graphics protocol: nothing to clear");
            }
        }

        Ok(())
    }

    /// Check if the protocol requires terminal clearing (Kitty and iTerm2)
    pub fn requires_terminal_clear(&self) -> bool {
        matches!(self.protocol, Protocol::Kitty | Protocol::Iterm2)
    }

    pub fn is_first_render(&self) -> bool {
        self.last_rendered_hash.is_none()
    }

    /// Clear a specific rectangular area by writing spaces
    /// This works for all protocols including iTerm2
    pub fn clear_area(&self, area: Rect) -> io::Result<()> {
        let stdout = io::stdout();
        let mut handle = stdout.lock();

        for row in area.y..area.y + area.height {
            queue!(handle, MoveTo(area.x, row))
                .map_err(|e| io::Error::other(format!("Failed to position cursor: {}", e)))?;
            handle.write_all(&vec![b' '; area.width as usize])?;
        }
        handle.flush()?;

        tracing::debug!("Cleared area: {:?}", area);
        Ok(())
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

    fn detection_context<'a>(term: &'a str, term_program: &'a str) -> ProtocolDetectionContext<'a> {
        ProtocolDetectionContext {
            term,
            term_program,
            warp_session: false,
            kitty_window_id: false,
            wt_session: false,
            wt_version: None,
            forced_protocol: None,
            chafa_available: false,
        }
    }

    #[test]
    fn test_protocol_detection() {
        let renderer = ImageRenderer::new();
        let _ = renderer.protocol();
    }

    #[test]
    fn test_unknown_terminal_with_chafa_uses_halfblocks() {
        let protocol = ImageRenderer::detect_protocol_from_env(ProtocolDetectionContext {
            chafa_available: true,
            ..detection_context("xterm-256color", "")
        });

        assert_eq!(protocol, Protocol::Halfblocks);
    }

    #[test]
    fn test_image_protocol_override_can_force_sixel() {
        let protocol = ImageRenderer::detect_protocol_from_env(ProtocolDetectionContext {
            forced_protocol: Some("sixel"),
            chafa_available: true,
            ..detection_context("xterm-256color", "")
        });

        assert_eq!(protocol, Protocol::Sixel);
    }

    #[test]
    fn test_macos_terminal_protocols_are_unchanged() {
        let terminal_app = ImageRenderer::detect_protocol_from_env(detection_context(
            "xterm-256color",
            "Apple_Terminal",
        ));
        let iterm2 = ImageRenderer::detect_protocol_from_env(detection_context(
            "xterm-256color",
            "iTerm.app",
        ));

        assert_eq!(terminal_app, Protocol::Halfblocks);
        assert_eq!(iterm2, Protocol::Iterm2);
    }

    #[test]
    fn test_windows_terminal_protocols_are_unchanged() {
        let modern_windows_terminal =
            ImageRenderer::detect_protocol_from_env(ProtocolDetectionContext {
                wt_session: true,
                wt_version: Some("1.22.10352.0"),
                chafa_available: true,
                ..detection_context("xterm-256color", "")
            });
        let older_windows_terminal =
            ImageRenderer::detect_protocol_from_env(ProtocolDetectionContext {
                wt_session: true,
                wt_version: Some("1.21.999.0"),
                chafa_available: true,
                ..detection_context("xterm-256color", "")
            });

        assert_eq!(modern_windows_terminal, Protocol::Iterm2);
        assert_eq!(older_windows_terminal, Protocol::Halfblocks);
    }

    #[test]
    fn test_rio_uses_kitty_protocol() {
        let protocol =
            ImageRenderer::detect_protocol_from_env(detection_context("xterm-256color", "Rio"));

        assert_eq!(protocol, Protocol::Kitty);
    }

    #[test]
    fn test_kitty_render_transmits_without_implicit_display() {
        let img = image::DynamicImage::new_rgba8(1, 1);
        let mut cursor = std::io::Cursor::new(Vec::new());
        img.write_to(&mut cursor, image::ImageFormat::Png)
            .expect("test image should encode as png");
        let png = cursor.into_inner();

        let mut renderer = ImageRenderer::new();
        let sequences = renderer
            .render_kitty(
                &png,
                Rect {
                    x: 10,
                    y: 5,
                    width: 40,
                    height: 20,
                },
            )
            .expect("kitty render should produce escape sequences");

        assert!(
            sequences.contains("a=t,t=d,f=100"),
            "image data should be transmitted without implicit display"
        );
        assert!(
            !sequences.contains("a=T"),
            "transmit-and-display creates an extra misplaced Kitty placement"
        );
        assert!(
            sequences.contains("a=p,i=1"),
            "image should be displayed only by the explicit placement"
        );
    }

    #[test]
    fn test_error_display() {
        let err = ImageError::ChafaNotInstalled;
        let lines = err.to_lines();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_calculate_display_size() {
        let renderer = ImageRenderer::new();
        // Test with 40x20 area (accounting for 3-cell margins on each side)
        // Available space: 40 - 6 = 34 cols, 20 - 6 = 14 rows
        let (cols, rows) = renderer.calculate_display_size(1920, 1080, 34, 14);
        assert!(cols > 0);
        assert!(rows > 0);
        assert!(cols <= 34); // 40 - 6 for margins
        assert!(rows <= 14); // 20 - 6 for margins
    }

    #[test]
    fn test_portrait_display_size_accounts_for_terminal_cell_shape() {
        let renderer = ImageRenderer::new();
        let (cols, rows) = renderer.calculate_display_size(600, 900, 40, 20);

        assert_eq!((cols, rows), (27, 20));
    }
}
