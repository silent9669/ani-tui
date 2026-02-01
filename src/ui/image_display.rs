use std::env;

#[derive(Debug, Clone, PartialEq)]
pub enum ImageProtocol {
    Kitty,  // WezTerm, Windows Terminal, Alacritty, Kitty
    ITerm2, // iTerm2
    None,   // Plain terminals (Terminal.app, PowerShell)
}

pub fn detect_protocol() -> ImageProtocol {
    // Check for WezTerm
    if env::var("TERM_PROGRAM").unwrap_or_default() == "WezTerm" {
        return ImageProtocol::Kitty;
    }

    // Check for iTerm2
    if env::var("TERM_PROGRAM").unwrap_or_default() == "Apple_Terminal" {
        // iTerm2 sets this differently
        if env::var("ITERM_PROFILE").is_ok() {
            return ImageProtocol::ITerm2;
        }
    }

    // Check for Kitty terminal
    if env::var("TERM").unwrap_or_default().contains("kitty") {
        return ImageProtocol::Kitty;
    }

    // Check for Windows Terminal or modern terminals
    if env::var("WT_SESSION").is_ok() {
        return ImageProtocol::Kitty;
    }

    // Check for COLORTERM=truecolor which often indicates Kitty-compatible terminals
    if env::var("COLORTERM").unwrap_or_default() == "truecolor" {
        return ImageProtocol::Kitty;
    }

    // Check for Alacritty
    if env::var("ALACRITTY_LOG").is_ok() || env::var("ALACRITTY_SOCKET").is_ok() {
        return ImageProtocol::Kitty;
    }

    ImageProtocol::None
}

pub fn supports_images() -> bool {
    let protocol = detect_protocol();
    matches!(protocol, ImageProtocol::Kitty | ImageProtocol::ITerm2)
}

pub fn display_image_placeholder(width: u32, height: u32) -> String {
    let protocol = detect_protocol();
    match protocol {
        ImageProtocol::Kitty | ImageProtocol::ITerm2 => {
            format!(
                "\x1b[?25l\x1b[7m[Image: {}x{} - loading...]\x1b[0m\x1b[?25h",
                width, height
            )
        }
        ImageProtocol::None => {
            format!("\n\n    ┌─────────────────────────────┐\n    │                             │\n    │       [ IMAGE PREVIEW ]      │\n    │                             │\n    │   {} x {} pixels          │\n    │                             │\n    │   (Terminal doesn't support │\n    │    inline images)           │\n    │                             │\n    └─────────────────────────────┘\n\n", width, height)
        }
    }
}

pub fn encode_kitty_image(image_data: &[u8], width: u32, height: u32) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    if image_data.is_empty() {
        return display_image_placeholder(width, height);
    }

    // Encode image to base64
    let encoded = STANDARD.encode(image_data);

    // Calculate actual display size
    // Kitty uses cells, not pixels. Each cell is roughly 2 chars wide
    let cell_width = (width / 2).max(10).min(80) as usize;
    let cell_height = height.max(5).min(30) as usize;

    // Kitty transmission command
    // Use inline image transmission protocol
    format!(
        "\x1b_Ga=t,f=100,s={},v={},m=1;{};\x1b\\\x1b[?25l",
        cell_width, cell_height, encoded
    )
}

pub fn encode_iterm_image(image_data: &[u8], width: u32, height: u32) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    if image_data.is_empty() {
        return display_image_placeholder(width, height);
    }

    let encoded = STANDARD.encode(image_data);
    let cell_width = (width / 2).max(10).min(80) as usize;
    let cell_height = height.max(5).min(30) as usize;

    // iTerm2 inline image protocol
    format!(
        "\x1331337;File=name=image.png;size={};width={};height={};inline=1:{}\x07\x1b[?25l",
        image_data.len(),
        cell_width,
        cell_height,
        encoded
    )
}

pub fn encode_image_for_display(image_data: &[u8], width: u32, height: u32) -> String {
    let protocol = detect_protocol();
    match protocol {
        ImageProtocol::Kitty => encode_kitty_image(image_data, width, height),
        ImageProtocol::ITerm2 => encode_iterm_image(image_data, width, height),
        ImageProtocol::None => display_image_placeholder(width, height),
    }
}

pub fn clear_image() -> String {
    let protocol = detect_protocol();
    match protocol {
        ImageProtocol::Kitty => "\x1b_Ga=d\x1b\\".to_string(),
        ImageProtocol::ITerm2 => "\x1331337;File=;\x07".to_string(),
        ImageProtocol::None => String::new(),
    }
}
