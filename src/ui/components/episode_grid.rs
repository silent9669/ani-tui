use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub struct EpisodeGrid;

pub struct EpisodeGridConfig {
    pub cell_width: usize,
    pub cell_spacing: usize,
    pub max_cols: usize,
    pub show_page_info: bool,
    pub center_grid: bool,
    pub left_padding_offset: usize,
    pub enable_filter: bool,
    pub enable_pagination: bool,
    pub page_size: usize,
    pub current_page: usize,
}

impl Default for EpisodeGridConfig {
    fn default() -> Self {
        Self {
            cell_width: 11,
            cell_spacing: 6,
            max_cols: 10,
            show_page_info: true,
            center_grid: true,
            left_padding_offset: 15,
            enable_filter: false,
            enable_pagination: false,
            page_size: 100,
            current_page: 0,
        }
    }
}

impl EpisodeGrid {
    pub fn render(
        episodes: &[crate::providers::Episode],
        current_idx: usize,
        selected_idx: usize,
        area: Rect,
        config: &EpisodeGridConfig,
    ) -> Vec<Line<'static>> {
        let available_width = area.width as usize;
        let available_height = area.height as usize;
        let max_visible_rows = available_height.saturating_sub(2);

        let cols = ((available_width.saturating_sub(4))
            / (config.cell_width + config.cell_spacing))
            .clamp(1, config.max_cols);

        let total_rows = episodes.len().div_ceil(cols);
        let rows_to_show = total_rows.min(max_visible_rows);

        let selected_row = selected_idx / cols;
        let scroll_offset = if total_rows > max_visible_rows {
            selected_row
                .saturating_sub(max_visible_rows / 2)
                .min(total_rows.saturating_sub(max_visible_rows))
        } else {
            0
        };

        let total_grid_width =
            cols * config.cell_width + (cols.saturating_sub(1)) * config.cell_spacing;
        let left_padding = if config.center_grid {
            ((available_width.saturating_sub(total_grid_width)) / 2) + config.left_padding_offset
        } else {
            2
        };

        let vertical_padding = (max_visible_rows.saturating_sub(rows_to_show)) / 2;
        let mut lines: Vec<Line> = Vec::new();

        for _ in 0..vertical_padding {
            lines.push(Line::from(""));
        }

        for row in 0..rows_to_show {
            let actual_row = row + scroll_offset;
            let mut row_spans: Vec<Span> = Vec::new();

            if left_padding > 0 {
                row_spans.push(Span::raw(" ".repeat(left_padding)));
            }

            for col in 0..cols {
                let idx = actual_row * cols + col;

                if let Some(episode) = episodes.get(idx) {
                    let is_selected = idx == selected_idx;
                    let is_current = idx == current_idx;

                    let ep_display = format!(" {:>4} ", episode.number);

                    let (bg_color, fg_color) = if is_selected {
                        (Color::Yellow, Color::Black)
                    } else if is_current {
                        (Color::Red, Color::White)
                    } else {
                        (Color::DarkGray, Color::White)
                    };

                    let cell_style = Style::default()
                        .fg(fg_color)
                        .bg(bg_color)
                        .add_modifier(Modifier::BOLD);

                    if col > 0 {
                        row_spans.push(Span::raw(" ".repeat(config.cell_spacing)));
                    }

                    row_spans.push(Span::styled(format!("[{}]", ep_display), cell_style));
                } else {
                    if col > 0 {
                        row_spans.push(Span::raw(" ".repeat(config.cell_spacing)));
                    }
                    row_spans.push(Span::raw(" ".repeat(config.cell_width)));
                }
            }

            lines.push(Line::from(row_spans));
        }

        while lines.len() < max_visible_rows {
            lines.push(Line::from(""));
        }

        lines
    }

    pub fn render_simple(
        episodes: &[crate::providers::Episode],
        current_idx: usize,
        selected_idx: usize,
        area: Rect,
    ) -> Vec<Line<'static>> {
        let config = EpisodeGridConfig {
            cell_spacing: 4,
            max_cols: 8,
            show_page_info: false,
            center_grid: true,
            left_padding_offset: 0,
            enable_filter: false,
            enable_pagination: false,
            ..Default::default()
        };
        Self::render(episodes, current_idx, selected_idx, area, &config)
    }

    pub fn render_fullscreen(
        episodes: &[crate::providers::Episode],
        current_idx: usize,
        selected_idx: usize,
        area: Rect,
    ) -> Vec<Line<'static>> {
        let config = EpisodeGridConfig {
            cell_spacing: 6,
            max_cols: 10,
            show_page_info: true,
            center_grid: true,
            left_padding_offset: 15,
            enable_filter: true,
            enable_pagination: true,
            page_size: 100,
            ..Default::default()
        };
        Self::render(episodes, current_idx, selected_idx, area, &config)
    }

    pub fn calculate_cols(area_width: usize, config: &EpisodeGridConfig) -> usize {
        ((area_width.saturating_sub(4)) / (config.cell_width + config.cell_spacing))
            .clamp(1, config.max_cols)
    }

    pub fn move_up(selected: usize, cols: usize, _total: usize) -> usize {
        if selected >= cols {
            selected - cols
        } else {
            selected
        }
    }

    pub fn move_down(selected: usize, cols: usize, total: usize) -> usize {
        if selected + cols < total {
            selected + cols
        } else {
            selected
        }
    }

    pub fn move_left(selected: usize) -> usize {
        if selected > 0 {
            selected - 1
        } else {
            selected
        }
    }

    pub fn move_right(selected: usize, total: usize) -> usize {
        if selected < total.saturating_sub(1) {
            selected + 1
        } else {
            selected
        }
    }

    pub fn filter_episodes<'a>(
        episodes: &'a [crate::providers::Episode],
        filter: &str,
    ) -> Vec<(usize, &'a crate::providers::Episode)> {
        if filter.is_empty() {
            return episodes.iter().enumerate().collect();
        }
        let filter_lower = filter.to_lowercase();
        episodes
            .iter()
            .enumerate()
            .filter(|(_, ep)| {
                let ep_str = format!("{}", ep.number);
                ep_str.contains(&filter_lower)
                    || ep
                        .title
                        .as_ref()
                        .map(|t| t.to_lowercase().contains(&filter_lower))
                        .unwrap_or(false)
            })
            .collect()
    }
}
