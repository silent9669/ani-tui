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

pub struct ImageRenderer {
    protocol: Protocol,
    sixel_cache: Option<Vec<u8>>,
    last_kitty_image_id: Option<u32>,
    last_rendered_hash: Option<u64>,
    last_rendered_area: Option<Rect>,
    last_image_data: Option<Vec<u8>>,
    last_render_time: Instant,
    last_halfblocks_lines: Option<Vec<Line<'static>>>,
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
        }
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

        tracing::debug!(
            "Terminal detection - TERM: {}, TERM_PROGRAM: {}, WARP_SESSION: {}, KITTY_WINDOW_ID: {}, WT_SESSION: {}",
            term,
            term_program,
            warp_session,
            kitty_window_id,
            wt_session
        );

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

        // 4. WezTerm - uses iTerm2 protocol
        if term_program == "WezTerm" {
            tracing::info!("Detected WezTerm - using iTerm2 protocol");
            return Protocol::Iterm2;
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

        // 7. Windows Terminal - Check for iTerm2 support or fallback to Halfblocks
        if wt_session {
            if let Ok(version) = std::env::var("WT_VERSION") {
                if Self::is_windows_terminal_modern(&version) {
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

        // 8. Other terminals - use Sixel if available, else Halfblocks
        if Self::is_chafa_available() {
            tracing::info!("Using Sixel protocol via chafa as fallback");
            return Protocol::Sixel;
        }

        tracing::info!("Using Halfblocks fallback");
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
    /// Returns Ok(None) if image was rendered successfully
    /// Returns Err if rendering failed
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
            Protocol::Iterm2 | Protocol::Sixel => true,
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
                    return Ok(Some(lines.clone()));
                }
            }
            return Ok(None);
        }

        tracing::debug!("Rendering image (hash: {}, area: {:?})", current_hash, area);

        // Render based on protocol
        let result = match self.protocol {
            Protocol::Kitty => {
                self.render_kitty(image_data, area)?;
                Ok(None)
            }
            Protocol::Iterm2 => {
                self.render_iterm2(image_data, area)?;
                Ok(None)
            }
            Protocol::Sixel => {
                self.render_sixel(image_data, area)?;
                Ok(None)
            }
            Protocol::Halfblocks | Protocol::None => {
                let lines = self.render_halfblocks(image_data, area)?;
                self.last_halfblocks_lines = Some(lines.clone());
                Ok(Some(lines))
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

    /// Render using Kitty graphics protocol with cursor-relative positioning
    ///
    /// Uses fixed image ID to prevent accumulation and centers image in the allocated area
    fn render_kitty(&mut self, image_data: &[u8], area: Rect) -> Result<(), ImageError> {
        // Use fixed image ID (always 1) to prevent accumulation and stacking
        let image_id = 1u32;

        // Get image dimensions
        let (img_width, img_height) = self.get_image_dimensions(image_data)?;

        // Calculate display size to fit within area while maintaining aspect ratio
        let (display_cols, display_rows) = self.calculate_display_size(
            img_width,
            img_height,
            area.width as u32,
            area.height as u32,
        );

        let stdout = io::stdout();
        let mut handle = stdout.lock();

        // Clear ALL previous images to prevent stacking/duplication
        // q=2: suppress all responses to prevent breaking keyboard input
        let clear_all_cmd = "\x1b_Ga=d,d=A,q=2\x1b\\";
        handle
            .write_all(clear_all_cmd.as_bytes())
            .map_err(|e| ImageError::RenderFailed(e.to_string()))?;

        // Send image data in chunks
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
                // t=d: direct transmission (data in payload)
                // f=100: PNG format (we send raw image data)
                // a=T: transmit action
                // m=1: more chunks coming
                // q=2: suppress all responses to prevent breaking keyboard input
                format!(
                    "a=T,t=d,f=100,i={},s={},v={},m={},q=2",
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

        // Calculate center position within the area
        let start_x = area.x + (area.width.saturating_sub(display_cols as u16)) / 2;
        let start_y = area.y + (area.height.saturating_sub(display_rows as u16)) / 2;

        // Position cursor at the calculated center position
        queue!(handle, MoveTo(start_x, start_y))
            .map_err(|e| ImageError::RenderFailed(format!("Failed to position cursor: {}", e)))?;

        // Create placement at cursor position (no U/V parameters)
        // C=1: do not move cursor after placement
        // q=2: suppress all responses to prevent breaking keyboard input
        let place_cmd = format!(
            "\x1b_Ga=p,i={},c={},r={},C=1,q=2\x1b\\",
            image_id, display_cols, display_rows
        );
        handle
            .write_all(place_cmd.as_bytes())
            .map_err(|e| ImageError::RenderFailed(e.to_string()))?;

        handle
            .flush()
            .map_err(|e| ImageError::RenderFailed(e.to_string()))?;

        // Update state tracking
        self.last_kitty_image_id = Some(image_id);

        tracing::debug!(
            "Rendered image {} via Kitty protocol at ({},{}) size {}x{} (centered in area {:?})",
            image_id,
            start_x,
            start_y,
            display_cols,
            display_rows,
            area
        );

        Ok(())
    }

    fn render_iterm2(&self, image_data: &[u8], area: Rect) -> Result<(), ImageError> {
        const MARGIN: u16 = 3;
        const SIZE_INCREASE: f32 = 2.5;

        if let Some(ref last_data) = self.last_image_data {
            if last_data == image_data && self.last_rendered_area == Some(area) {
                tracing::debug!("Skipping iTerm2 render - same image and area");
                return Ok(());
            }
        }

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

        let start_x = area.x + MARGIN + (available_width - display_cols as u16) / 2;
        let start_y = area.y + MARGIN + (available_height - display_rows as u16) / 2;

        let base64_data = general_purpose::STANDARD.encode(image_data);

        let osc = format!(
            "\x1b]1337;File=inline=1;size={};width={};height={};preserveAspectRatio=1;doNotMoveCursor=1:{}\x07",
            image_data.len(),
            display_cols,
            display_rows,
            base64_data
        );

        let stdout = io::stdout();
        let mut handle = stdout.lock();

        let spaces = vec![b' '; area.width as usize];
        for row in area.y..area.y + area.height {
            queue!(handle, MoveTo(area.x, row)).map_err(|e| {
                ImageError::RenderFailed(format!("Failed to position cursor: {}", e))
            })?;
            handle
                .write_all(&spaces)
                .map_err(|e| ImageError::RenderFailed(e.to_string()))?;
        }

        handle
            .flush()
            .map_err(|e| ImageError::RenderFailed(e.to_string()))?;

        queue!(handle, MoveTo(start_x, start_y))
            .map_err(|e| ImageError::RenderFailed(format!("Failed to position cursor: {}", e)))?;

        handle
            .write_all(osc.as_bytes())
            .map_err(|e| ImageError::RenderFailed(e.to_string()))?;
        handle
            .flush()
            .map_err(|e| ImageError::RenderFailed(e.to_string()))?;

        tracing::debug!(
            "Rendered image via iTerm2 protocol at ({}, {}) size {}x{} cells (with {} cell margins)",
            start_x,
            start_y,
            display_cols,
            display_rows,
            MARGIN
        );

        Ok(())
    }

    /// Render using Sixel via chafa
    fn render_sixel(&mut self, image_data: &[u8], area: Rect) -> Result<(), ImageError> {
        // Check cache - but invalidate if area size changed
        if let Some(ref cached) = self.sixel_cache {
            // Check if cached sixel is for the same area size
            if let Some(last_area) = self.last_rendered_area {
                if last_area.width == area.width && last_area.height == area.height {
                    let stdout = io::stdout();
                    let mut handle = stdout.lock();

                    // CRITICAL FIX: Position cursor BEFORE writing cached sixel data
                    queue!(handle, MoveTo(area.x, area.y)).map_err(|e| {
                        ImageError::RenderFailed(format!("Failed to position cursor: {}", e))
                    })?;

                    handle
                        .write_all(cached)
                        .map_err(|e| ImageError::RenderFailed(e.to_string()))?;
                    handle
                        .flush()
                        .map_err(|e| ImageError::RenderFailed(e.to_string()))?;
                    return Ok(());
                }
            }
            // Area size changed, invalidate cache
            self.sixel_cache = None;
        }

        // Calculate size accounting for borders (2 cells on each side)
        let render_width = area.width.saturating_sub(2);
        let render_height = area.height.saturating_sub(2);

        // Spawn chafa process with high-quality settings
        // Using --symbols all for best quality (as used in v2.0.0)
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
                "all", // High quality symbol set
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

        // Write sixel output directly to stdout
        // Position cursor first to ensure correct placement
        let stdout = io::stdout();
        let mut handle = stdout.lock();

        // Move cursor to top-left of image area
        queue!(handle, MoveTo(area.x, area.y))
            .map_err(|e| ImageError::RenderFailed(format!("Failed to position cursor: {}", e)))?;

        handle
            .write_all(&output.stdout)
            .map_err(|e| ImageError::RenderFailed(e.to_string()))?;
        handle
            .flush()
            .map_err(|e| ImageError::RenderFailed(e.to_string()))?;

        tracing::debug!(
            "Rendered image via Sixel protocol at ({}, {})",
            area.x,
            area.y
        );

        Ok(())
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
        let aspect_ratio = img_width as f32 / img_height as f32;

        // Use 100% of available space - maximize image size
        let available_cols = max_cols;
        let available_rows = max_rows;

        // Try to fit in max dimensions
        let mut cols = available_cols;
        let mut rows = (cols as f32 / aspect_ratio) as u32;

        // If too tall, scale down
        if rows > available_rows {
            rows = available_rows;
            cols = (rows as f32 * aspect_ratio) as u32;
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

    #[test]
    fn test_protocol_detection() {
        let renderer = ImageRenderer::new();
        let _ = renderer.protocol();
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
}
