use super::ascii_art::AsciiArt;
use super::components::LoadingSpinner;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub struct SplashScreen {
    frame_count: usize,
    loading_spinner: LoadingSpinner,
    loading_progress: u8,
    load_status: String,
}

impl Default for SplashScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl SplashScreen {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            loading_spinner: LoadingSpinner::new(),
            loading_progress: 0,
            load_status: "Initializing...".to_string(),
        }
    }

    pub fn tick(&mut self) {
        self.frame_count += 1;
        self.loading_spinner.tick();
    }

    pub fn set_progress(&mut self, progress: u8, status: &str) {
        self.loading_progress = progress.min(100);
        self.load_status = status.to_string();
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Length(12),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Percentage(25),
            ])
            .split(area);

        let banner_lines = AsciiArt::render_banner_colored();
        let banner_text = Text::from(banner_lines);
        let banner = Paragraph::new(banner_text).alignment(Alignment::Center);
        frame.render_widget(banner, chunks[1]);

        let version_style = Style::default().fg(Color::DarkGray);
        let version_text = Paragraph::new(format!("ani-tui v{}", env!("CARGO_PKG_VERSION")))
            .alignment(Alignment::Center)
            .style(version_style);
        frame.render_widget(version_text, chunks[2]);

        let spinner = AsciiArt::loading_spinner(self.frame_count);
        let status_text = format!("{} {}", spinner, self.load_status);
        let status = Paragraph::new(status_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(status, chunks[3]);

        let bar = Self::render_progress_bar(self.loading_progress, 50);
        let bar_color = if self.loading_progress < 30 {
            Color::Yellow
        } else if self.loading_progress < 70 {
            Color::Cyan
        } else {
            Color::Green
        };
        let loading_bar = Paragraph::new(bar)
            .alignment(Alignment::Center)
            .style(Style::default().fg(bar_color));
        frame.render_widget(loading_bar, chunks[4]);
    }

    fn render_progress_bar(progress: u8, width: usize) -> String {
        let filled = (progress as usize * width) / 100;
        let empty = width - filled;
        format!("┃{}{}┃", "█".repeat(filled), "░".repeat(empty))
    }

    pub fn is_complete(&self, elapsed_ms: u64) -> bool {
        elapsed_ms > 1500 || self.loading_progress >= 100
    }
}

pub struct PreviewPanel;

impl PreviewPanel {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        anime: Option<&crate::metadata::EnrichedAnime>,
        app: &mut crate::ui::app::App,
    ) {
        let block = Block::default().borders(Borders::ALL).title("Preview");

        if let Some(anime) = anime {
            let inner_area = block.inner(area);
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(inner_area);

            // Check if we switched to a different anime
            let current_anime_id = Some(anime.base.id.clone());
            if current_anime_id != *app.last_preview_anime_id() {
                // Anime changed - clear terminal graphics to prevent layering
                if app.image_renderer().requires_terminal_clear() {
                    let _ = app.image_renderer_mut().clear_terminal_graphics();
                }
                app.set_last_preview_anime_id(current_anime_id);
            }

            // Use current image data (single context)
            let has_image = app
                .current_image_data
                .as_ref()
                .map(|d: &Vec<u8>| !d.is_empty())
                .unwrap_or(false);

            if has_image {
                let image_data = app.current_image_data.clone();
                if let Some(data) = image_data {
                    app.render_image_with_ratatui(frame, chunks[0], &data);
                } else {
                    Self::render_image_placeholder(frame, chunks[0], false);
                }
            } else {
                Self::render_image_placeholder(frame, chunks[0], false);
            }

            let mut lines: Vec<Line> = Vec::new();

            // Title
            lines.push(Line::from(vec![Span::styled(
                anime.base.title.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));

            // Language badge
            let lang_color = match anime.base.language {
                crate::providers::Language::English => Color::Blue,
                crate::providers::Language::Vietnamese => Color::Yellow,
            };
            lines.push(Line::from(vec![Span::styled(
                format!("[{}]", anime.base.language),
                Style::default().fg(lang_color),
            )]));

            lines.push(Line::from(""));

            // Always show episodes from base info first
            if let Some(eps) = anime.base.total_episodes {
                lines.push(Line::from(vec![
                    Span::raw("Episodes: "),
                    Span::styled(eps.to_string(), Style::default().fg(Color::Green)),
                ]));
            } else if let Some(ref metadata) = anime.metadata {
                if let Some(episodes) = metadata.episode_count {
                    lines.push(Line::from(vec![
                        Span::raw("Episodes: "),
                        Span::styled(episodes.to_string(), Style::default().fg(Color::Green)),
                    ]));
                }
            }

            // Rating from metadata
            if let Some(ref metadata) = anime.metadata {
                if let Some(rating) = metadata.rating {
                    let stars = Self::render_stars(rating);
                    lines.push(Line::from(vec![
                        Span::raw("Rating: "),
                        Span::styled(stars, Style::default().fg(Color::Yellow)),
                        Span::raw(format!(" {:.1}/10", rating as f64 / 10.0)),
                    ]));
                }

                // Genres
                if !metadata.genres.is_empty() {
                    let genre_text = metadata.genres.join(", ");
                    lines.push(Line::from(vec![
                        Span::raw("Genres: "),
                        Span::styled(genre_text, Style::default().fg(Color::Magenta)),
                    ]));
                }

                lines.push(Line::from(""));

                // Description from metadata
                if let Some(ref desc) = metadata.description {
                    let clean_desc = desc
                        .replace("<br>", "\n")
                        .replace("<br/>", "\n")
                        .replace("</i>", "")
                        .replace("<i>", "")
                        .replace("<b>", "")
                        .replace("</b>", "");

                    for line in clean_desc.lines() {
                        if !line.trim().is_empty() {
                            lines.push(Line::from(line.to_string()));
                        }
                    }
                }
            } else {
                // No metadata yet
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Loading metadata...",
                    Style::default().fg(Color::Gray),
                )));

                // Show base synopsis if available
                if let Some(ref synopsis) = anime.base.synopsis {
                    lines.push(Line::from(""));
                    for line in synopsis.lines() {
                        if !line.trim().is_empty() {
                            lines.push(Line::from(line.to_string()));
                        }
                    }
                }
            }

            let info_block = Block::default().borders(Borders::TOP);
            let paragraph = Paragraph::new(Text::from(lines))
                .block(info_block)
                .wrap(Wrap { trim: false });

            frame.render_widget(paragraph, chunks[1]);
            frame.render_widget(block, area);
        } else {
            let paragraph = Paragraph::new("Select an anime to see details")
                .block(block)
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        }
    }

    fn render_stars(rating: i64) -> String {
        let stars = rating / 20; // 0-100 to 0-5
        let filled = "★".repeat(stars as usize);
        let empty = "☆".repeat(5 - stars as usize);
        format!("{}{}", filled, empty)
    }

    fn render_image_placeholder(frame: &mut Frame, area: Rect, has_image: bool) {
        let image_lines: Vec<Line> = if has_image {
            vec![
                Line::from(vec![Span::styled(
                    "┌─────────────────────────────────────┐",
                    Style::default().fg(Color::Blue),
                )]),
                Line::from(vec![Span::styled(
                    "│         📷 COVER IMAGE              │",
                    Style::default().fg(Color::Cyan),
                )]),
                Line::from(vec![Span::styled(
                    "│                                     │",
                    Style::default().fg(Color::Blue),
                )]),
                Line::from(vec![Span::styled(
                    "│     [Image Preview Available]       │",
                    Style::default().fg(Color::Gray),
                )]),
                Line::from(vec![Span::styled(
                    "│                                     │",
                    Style::default().fg(Color::Blue),
                )]),
                Line::from(vec![Span::styled(
                    "└─────────────────────────────────────┘",
                    Style::default().fg(Color::Blue),
                )]),
            ]
        } else {
            vec![
                Line::from(vec![Span::styled(
                    "┌─────────────────────────────────────┐",
                    Style::default().fg(Color::Gray),
                )]),
                Line::from(vec![Span::styled(
                    "│                                     │",
                    Style::default().fg(Color::Gray),
                )]),
                Line::from(vec![Span::styled(
                    "│         [No Image Available]        │",
                    Style::default().fg(Color::DarkGray),
                )]),
                Line::from(vec![Span::styled(
                    "│                                     │",
                    Style::default().fg(Color::Gray),
                )]),
                Line::from(vec![Span::styled(
                    "└─────────────────────────────────────┘",
                    Style::default().fg(Color::Gray),
                )]),
            ]
        };

        let image_widget = Paragraph::new(Text::from(image_lines)).alignment(Alignment::Center);
        frame.render_widget(image_widget, area);
    }
}

pub struct SearchOverlay {
    pub query: String,
    pub results: Vec<crate::metadata::EnrichedAnime>,
    pub selected_index: usize,
    pub is_searching: bool,
}

impl Default for SearchOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchOverlay {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected_index: 0,
            is_searching: false,
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        sources: &[crate::providers::Language],
        current_page: usize,
        total_pages: usize,
    ) {
        let layout = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .margin(1)
            .constraints([
                ratatui::layout::Constraint::Length(2), // Caption
                ratatui::layout::Constraint::Length(3), // Search input
                ratatui::layout::Constraint::Min(0),    // Results
            ])
            .split(area);

        // Centered caption at top
        let caption = Paragraph::new("Search for Anime & Films")
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(caption, layout[0]);

        // Search input
        let search_block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Search ({} active sources)", sources.len()));

        let search_text = if self.is_searching {
            format!("{} ▐", self.query)
        } else {
            format!("{}_", self.query)
        };

        let search_input = Paragraph::new(search_text).block(search_block);
        frame.render_widget(search_input, layout[1]);

        let results_block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Results ({})", self.results.len()));

        let mut lines: Vec<Line> = Vec::new();

        for (idx, anime) in self.results.iter().enumerate() {
            let is_selected = idx == self.selected_index;

            let lang_color = match anime.base.language {
                crate::providers::Language::English => Color::Blue,
                crate::providers::Language::Vietnamese => Color::Yellow,
            };

            let flag = match anime.base.language {
                crate::providers::Language::English => "🇺🇸",
                crate::providers::Language::Vietnamese => "🇻🇳",
            };

            let badge = Span::styled(format!("{} ", flag), Style::default().fg(lang_color));

            let title_style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let prefix = if is_selected { "▶ " } else { "  " };

            lines.push(Line::from(vec![
                Span::raw(prefix),
                badge,
                Span::styled(&anime.base.title, title_style),
            ]));
        }

        if self.results.is_empty() && !self.query.is_empty() && !self.is_searching {
            lines.push(Line::from("No results found"));
        } else if self.query.is_empty() {
            lines.push(Line::from("Type to search..."));
        }

        // Add pagination info if multiple pages exist
        if total_pages > 1 {
            lines.push(Line::from(""));
            lines.push(
                Line::from(vec![
                    Span::raw("--- Page "),
                    Span::styled(
                        current_page.to_string(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" of "),
                    Span::styled(total_pages.to_string(), Style::default().fg(Color::Cyan)),
                    Span::raw(" ---"),
                ])
                .alignment(Alignment::Center),
            );
        }

        let results_list = Paragraph::new(Text::from(lines))
            .block(results_block)
            .wrap(Wrap { trim: true });

        frame.render_widget(results_list, layout[2]);
    }
}

pub struct SourceSelectModal;

impl SourceSelectModal {
    pub fn render(
        frame: &mut Frame,
        frame_area: Rect,
        sources: &[(String, crate::providers::Language, bool)],
        selected: usize,
    ) {
        frame.render_widget(ratatui::widgets::Clear, frame_area);

        let content_height = (sources.len() + 3) as u16;
        let modal_height = content_height.min(frame_area.height.saturating_sub(4));
        let modal_width = ((frame_area.width as f32 * 0.4).clamp(40.0, 50.0)) as u16;

        let modal_area = Rect {
            x: frame_area.x + (frame_area.width.saturating_sub(modal_width)) / 2,
            y: frame_area.y + (frame_area.height.saturating_sub(modal_height)) / 2,
            width: modal_width,
            height: modal_height,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Select Source (Enter to confirm)");

        let inner_area = block.inner(modal_area);
        let modal_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(inner_area);

        let caption = Paragraph::new("Select subtitle language:")
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(caption, modal_chunks[0]);

        let mut lines: Vec<Line> = Vec::new();

        for (idx, (name, lang, enabled)) in sources.iter().enumerate() {
            let is_selected = idx == selected;
            let prefix = if is_selected { "> " } else { "  " };
            let radio = if *enabled { "(◉)" } else { "(○)" };

            let lang_badge = match lang {
                crate::providers::Language::English => {
                    Span::styled("[EN]", Style::default().fg(Color::Blue))
                }
                crate::providers::Language::Vietnamese => {
                    Span::styled("[VN]", Style::default().fg(Color::Yellow))
                }
            };

            let style = if is_selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            lines.push(Line::from(vec![
                Span::styled(format!("{}{} ", prefix, radio), style),
                lang_badge,
                Span::raw(" "),
                Span::styled(name.clone(), style),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Only one source can be active",
            Style::default().fg(Color::Gray),
        )));

        let paragraph = Paragraph::new(Text::from(lines)).alignment(Alignment::Center);

        frame.render_widget(paragraph, modal_chunks[1]);
        frame.render_widget(block, modal_area);
    }
}
