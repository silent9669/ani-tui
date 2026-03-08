use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

pub struct AsciiArt;

impl AsciiArt {
    pub fn banner() -> Vec<&'static str> {
        vec![
            "",
            r"  /$$$$$$  /$$   /$$ /$$$$$$    /$$$$$$$$ /$$   /$$ /$$$$$$",
            r" /$$__  $$| $$$ | $$|_  $$_/   |__  $$__/| $$  | $$|_  $$_/",
            r"| $$  \ $$| $$$$| $$  | $$        | $$   | $$  | $$  | $$  ",
            r"| $$$$$$$$| $$ $$ $$  | $$ /$$$$$$| $$   | $$  | $$  | $$  ",
            r"| $$__  $$| $$  $$$$  | $$|______/| $$   | $$  | $$  | $$  ",
            r"| $$  | $$| $$\  $$$  | $$        | $$   | $$  | $$  | $$  ",
            r"| $$  | $$| $$ \  $$ /$$$$$$      | $$   |  $$$$$$/ /$$$$$$",
            r"|__/  |__/|__/  \__/|______/      |__/    \______/ |______/",
            "",
            "",
        ]
    }

    pub fn subtitle() -> &'static str {
        "Terminal UI for Anime Streaming"
    }

    pub fn render_banner_colored() -> Vec<Line<'static>> {
        let lines = Self::banner();
        let colors = vec![
            Color::Gray,
            Color::Cyan,
            Color::Magenta,
            Color::Cyan,
            Color::Magenta,
            Color::Cyan,
            Color::Magenta,
            Color::Cyan,
            Color::Magenta,
            Color::Gray,
            Color::Gray,
        ];

        lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let color = colors.get(i).copied().unwrap_or(Color::White);
                Line::from(Span::styled(*line, Style::default().fg(color)))
            })
            .collect()
    }

    pub fn render_full_intro() -> Vec<Line<'static>> {
        let mut lines = Self::render_banner_colored();
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            Self::subtitle(),
            Style::default().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
        lines
    }

    pub fn progress_bar(progress_pct: u8, width: usize) -> String {
        let filled = (progress_pct as usize * width) / 100;
        let empty = width - filled;
        format!(
            "[{}{}] {}%",
            "█".repeat(filled),
            "░".repeat(empty),
            progress_pct
        )
    }

    pub fn loading_spinner(frame: usize) -> &'static str {
        let frames = ["◐", "◓", "◑", "◒"];
        frames[frame % frames.len()]
    }
}
