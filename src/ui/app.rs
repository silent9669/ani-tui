use crate::config::Config;
use crate::db::Database;
use crate::image::ImagePipeline;
use crate::metadata::{EnrichedAnime, MetadataCache};
use crate::player::Player;
use crate::providers::{AnimeProvider, Episode, Language, ProviderRegistry};
use crate::ui::components::{LoadingSpinner, Toast};
use crate::ui::image_renderer::ImageRenderer;
use crate::ui::modern_components::{PreviewPanel, SearchOverlay, SplashScreen};
use crate::ui::player_controller::{ControlAction, PlayerController, PlayerState};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, DisableLineWrap, EnableLineWrap, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    Splash,
    SourceSelect,
    Home,
    Search,
    EpisodeSelect,
    Player,
}

#[allow(dead_code)]
pub struct App {
    config: Config,
    db: Arc<Database>,
    providers: ProviderRegistry,
    #[allow(dead_code)]
    player: Player,
    current_screen: Screen,
    should_quit: bool,

    // New components
    splash_screen: SplashScreen,
    splash_start: Instant,
    metadata_cache: MetadataCache,
    image_pipeline: ImagePipeline,
    player_controller: PlayerController,

    // Source selection - only one source at a time
    selected_source: Language,
    selected_source_idx: usize,
    show_source_modal: bool,

    // Search state
    search_overlay: SearchOverlay,
    enriched_results: Vec<EnrichedAnime>,

    // Navigation
    selected_index: usize,

    // Selected anime and episodes
    selected_anime: Option<EnrichedAnime>,
    episodes: Vec<Episode>,
    #[allow(dead_code)]
    episode_list_state: ListState,

    // Continue watching
    continue_watching: Vec<crate::db::WatchHistory>,
    continue_watching_selected: usize,

    // Preloaded metadata for continue watching (anime_id -> total_episodes)
    continue_watching_metadata: std::collections::HashMap<String, u32>,

    // UI Components
    loading_spinner: LoadingSpinner,
    toast: Option<Toast>,

    // SINGLE image context - simplified architecture
    pub(crate) current_image_data: Option<Vec<u8>>,
    current_anime_id: Option<String>,
    current_sixel_cache: Option<String>,

    // Image transition tracking for smooth fades
    previous_image_data: Option<Vec<u8>>,
    transition_progress: f32, // 0.0 to 1.0
    in_transition: bool,
    last_image_render: Instant,

    // New image renderer with multi-protocol support
    image_renderer: ImageRenderer,

    // Async image loading state
    #[allow(clippy::type_complexity)]
    pending_image_load:
        Option<std::pin::Pin<Box<dyn std::future::Future<Output = Option<Vec<u8>>>>>>,
    image_loading: bool,

    // Preloaded images for smooth switching
    preloaded_images: std::collections::HashMap<String, Vec<u8>>, // anime_id -> image_data

    // Track previous screen for navigation
    previous_screen: Option<Screen>,

    // Track preloading status for splash screen
    preloaded_image_ids: Vec<String>,

    // Trigger preview load when entering search
    needs_preview_load: bool,

    // Trigger continue watching refresh when entering home
    needs_continue_watching_refresh: bool,

    source_modal_for_search: bool,

    show_episode_list: bool,
    episode_list_scroll: usize,
    episode_list_selected: usize,

    // Episode selection screen (Phase 3: grid layout with pagination)
    episode_filter: String,
    episode_filter_mode: bool,
    episode_selected_index: usize,
    episode_grid_columns: usize,
    episode_current_page: usize,
    episodes_per_page: usize,

    // Search debounce
    search_pending: bool,
    #[allow(dead_code)]
    last_keypress: Instant,

    // Image navigation debounce (prevents rapid re-renders when holding arrow keys)
    last_image_navigation: Instant,

    // Track last rendered anime ID in PreviewPanel to detect changes
    last_preview_anime_id: Option<String>,
}

impl App {
    pub async fn new(config: Config, db: Arc<Database>) -> Result<Self> {
        let providers = ProviderRegistry::new(&config);
        let player = Player::new();

        // Load continue watching - use a direct reference without Mutex since Database has internal locking
        let continue_watching = db.get_continue_watching(10).await.unwrap_or_default();

        // Setup image pipeline
        let image_pipeline = ImagePipeline::new(db.clone());

        // Preload all watch history images in background
        let images_to_preload: Vec<(String, String)> = continue_watching
            .iter()
            .filter(|h| !h.cover_url.is_empty())
            .map(|h| {
                (
                    format!("continue_watching_{}", h.anime_id),
                    h.cover_url.clone(),
                )
            })
            .collect();
        let preloaded_image_ids: Vec<String> =
            images_to_preload.iter().map(|(id, _)| id.clone()).collect();
        drop(image_pipeline.preload_images(images_to_preload));

        // Load first image immediately if available
        let mut current_image_data = None;
        if let Some(first) = continue_watching.first() {
            if !first.cover_url.is_empty() {
                let image_id = format!("continue_watching_{}", first.anime_id);
                match image_pipeline
                    .request_download(image_id, first.cover_url.clone())
                    .await
                {
                    Ok(data) => {
                        current_image_data = Some(data);
                    }
                    Err(e) => tracing::warn!("Failed to load first cover image: {}", e),
                }
            }
        }

        // Setup selected source - only one at a time
        let selected_source_idx = if config.sources.vietnamese { 1 } else { 0 };
        let selected_source = if config.sources.vietnamese {
            Language::Vietnamese
        } else {
            Language::English
        };

        // Database already has internal locking
        let metadata_cache = MetadataCache::new(db.clone());

        // Initialize new image renderer with multi-protocol support
        let image_renderer = ImageRenderer::new();

        Ok(Self {
            config,
            db,
            providers,
            player,
            current_screen: Screen::Splash,
            should_quit: false,
            splash_screen: SplashScreen::new(),
            splash_start: Instant::now(),
            metadata_cache,
            image_pipeline,
            player_controller: PlayerController::new(),
            selected_source,
            selected_source_idx,
            show_source_modal: false,
            search_overlay: SearchOverlay::new(),
            enriched_results: Vec::new(),
            selected_index: 0,
            selected_anime: None,
            episodes: Vec::new(),
            episode_list_state: ListState::default(),
            continue_watching,
            continue_watching_selected: 0,
            loading_spinner: LoadingSpinner::new(),
            toast: None,
            // Initialize single image context
            current_image_data,
            current_anime_id: None,
            current_sixel_cache: None,
            // Transition tracking
            previous_image_data: None,
            transition_progress: 0.0,
            in_transition: false,
            last_image_render: Instant::now(),
            // Image renderer
            image_renderer,
            previous_screen: None,
            preloaded_image_ids,
            needs_preview_load: false,
            needs_continue_watching_refresh: false,
            show_episode_list: false,
            episode_list_scroll: 0,
            episode_list_selected: 0,
            episode_filter: String::new(),
            episode_filter_mode: false,
            episode_selected_index: 0,
            episode_grid_columns: 6,
            episode_current_page: 0,
            episodes_per_page: 100,
            continue_watching_metadata: std::collections::HashMap::new(),
            search_pending: false,
            last_keypress: Instant::now(),
            last_image_navigation: Instant::now(),
            last_preview_anime_id: None,
            // Async image loading state
            pending_image_load: None,
            image_loading: false,
            // Preloaded images cache
            preloaded_images: std::collections::HashMap::new(),
            source_modal_for_search: false,
        })
    }

    pub fn set_initial_search(&mut self, query: String) {
        self.search_overlay.query = query;
        self.current_screen = Screen::Search;
    }

    pub async fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        // Disable line wrap to prevent auto-scrolling on Windows
        execute!(stdout, DisableLineWrap)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.run_app(&mut terminal).await;

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            EnableLineWrap
        )?;
        terminal.show_cursor()?;

        res
    }

    async fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        let mut last_tick = Instant::now();
        let tick_rate = std::time::Duration::from_millis(100);

        loop {
            terminal.draw(|f| self.draw(f))?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| std::time::Duration::from_secs(0));

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if key.code == KeyCode::Char('S') && self.current_screen == Screen::Home {
                            // Clear images before transitioning to source select
                            if self.image_renderer.requires_terminal_clear() {
                                drop(self.image_renderer.clear_terminal_graphics());
                            }
                            self.current_image_data = None;
                            self.current_anime_id = None;
                            self.previous_image_data = None;
                            self.in_transition = false;
                            self.image_renderer.clear_cache();
                            self.last_preview_anime_id = None;
                            self.current_screen = Screen::SourceSelect;
                            continue;
                        }

                        // Handle Shift+C for source toggle during search
                        if key.code == KeyCode::Char('C') && self.current_screen == Screen::Search {
                            self.show_source_modal = !self.show_source_modal;
                            continue;
                        }

                        self.handle_key(key.code).await?;
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.on_tick().await?;
                last_tick = Instant::now();
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        match self.current_screen {
            Screen::Splash => self.draw_splash(frame),
            Screen::SourceSelect => self.draw_source_select(frame),
            Screen::Home => self.draw_home(frame),
            Screen::Search => self.draw_search(frame),
            Screen::EpisodeSelect => self.draw_episode_select(frame),
            Screen::Player => self.draw_player(frame),
        }

        // Draw source modal if active
        if self.show_source_modal {
            self.draw_source_modal(frame);
        }

        // Draw toast if present
        if let Some(ref toast) = self.toast {
            self.draw_toast(frame, toast);
        }
    }

    fn draw_splash(&mut self, frame: &mut Frame) {
        let area = frame.size();
        self.splash_screen.render(frame, area);
    }

    fn draw_source_select(&mut self, frame: &mut Frame) {
        let area = frame.size();

        // Full-screen source select layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30), // Top padding
                Constraint::Length(3),      // Title
                Constraint::Length(6),      // Source options
                Constraint::Length(3),      // Help text
                Constraint::Percentage(30), // Bottom padding
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Select Source")
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(title, chunks[1]);

        // Source options
        let sources: Vec<(String, Language, bool)> = vec![
            (
                "AllAnime (English)".to_string(),
                Language::English,
                self.selected_source == Language::English,
            ),
            (
                "KKPhim (Vietnamese)".to_string(),
                Language::Vietnamese,
                self.selected_source == Language::Vietnamese,
            ),
        ];

        let mut lines: Vec<Line> = Vec::new();
        for (idx, (name, lang, enabled)) in sources.iter().enumerate() {
            let is_selected = idx == self.selected_source_idx;
            let prefix = if is_selected { "▶ " } else { "  " };
            let radio = if *enabled { "(◉)" } else { "(○)" };

            let lang_badge = match lang {
                Language::English => Span::styled("[EN]", Style::default().fg(Color::Blue)),
                Language::Vietnamese => Span::styled("[VN]", Style::default().fg(Color::Yellow)),
            };

            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            lines.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled(format!("{} ", radio), style),
                lang_badge,
                Span::raw(" "),
                Span::styled(name.clone(), style),
            ]));
            lines.push(Line::from(""));
        }

        let options = Paragraph::new(Text::from(lines)).alignment(Alignment::Center);
        frame.render_widget(options, chunks[2]);

        // Help text
        let help = Paragraph::new("↑/↓: Select | Enter: Confirm | Esc: Cancel")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(help, chunks[3]);
    }

    fn draw_home(&mut self, frame: &mut Frame) {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(0),    // Content area (full height)
                Constraint::Length(1), // Status bar
            ])
            .split(frame.size());

        // Content area - split into Continue Watching and Preview
        if !self.continue_watching.is_empty() {
            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[0]);

            // Continue watching list
            self.draw_continue_watching(frame, content_chunks[0]);

            // Preview panel for selected item
            self.draw_continue_watching_preview(frame, content_chunks[1]);
        } else {
            let no_history = Paragraph::new("No watch history yet.\nStart watching to see your progress here!\n\nPress Shift+S to search for anime.")
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(no_history, main_chunks[0]);
        }

        // Status bar at bottom
        let status_bar = Paragraph::new(
            "↑/↓: Navigate | Enter: Resume | Shift+D: Remove | Shift+S: Search | ESC: Quit",
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
        frame.render_widget(status_bar, main_chunks[1]);
    }

    fn draw_continue_watching(&self, frame: &mut Frame, area: Rect) {
        let mut history_lines: Vec<Line> = Vec::new();

        for (idx, history) in self.continue_watching.iter().enumerate() {
            let is_selected = idx == self.continue_watching_selected;

            let prefix = if is_selected { "▶ " } else { "  " };
            let title_style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };

            // Only show anime title in the list - larger text
            history_lines.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled(&history.title, title_style),
            ]));
        }

        let history_widget = Paragraph::new(Text::from(history_lines)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Continue Watching"),
        );

        frame.render_widget(history_widget, area);
    }

    fn draw_continue_watching_preview(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(history) = self
            .continue_watching
            .get(self.continue_watching_selected)
            .cloned()
        {
            let history_title = history.title.clone();
            let history_cover_url = history.cover_url.clone();
            let history_episode = history.episode_number;
            let history_episode_title = history.episode_title.clone();
            let history_provider = history.provider.clone();
            let history_position = history.position_seconds;
            let history_total = history.total_seconds;
            let history_updated = history.updated_at;
            let history_anime_id = history.anime_id.clone();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(60), // Image section
                    Constraint::Percentage(40), // Description section
                ])
                .margin(0) // Remove margin for full-size images
                .split(area);

            // Use current image data (single context)
            let has_image = self
                .current_image_data
                .as_ref()
                .map(|d| !d.is_empty())
                .unwrap_or(false);

            if has_image {
                if let Some(image_data) = self.current_image_data.clone() {
                    self.render_image_with_ratatui(frame, chunks[0], &image_data);
                } else {
                    let image_block = Block::default().borders(Borders::ALL).title("Cover Image");
                    let image_text = "[Image error]";
                    let image_widget = Paragraph::new(image_text)
                        .alignment(Alignment::Center)
                        .block(image_block);
                    frame.render_widget(image_widget, chunks[0]);
                }
            } else {
                // Show loading spinner when actively loading
                let image_block =
                    Block::default()
                        .borders(Borders::ALL)
                        .title(if self.image_loading {
                            "Loading Image..."
                        } else {
                            "Cover Image"
                        });

                let image_widget = if self.image_loading {
                    // Update spinner and render it
                    self.loading_spinner.tick();
                    let spinner_line = self.loading_spinner.render();
                    Paragraph::new(Text::from(vec![
                        Line::from(""),
                        spinner_line,
                        Line::from(""),
                    ]))
                    .alignment(Alignment::Center)
                    .block(image_block)
                } else {
                    let image_text = if !history_cover_url.is_empty() {
                        "[Loading image...]"
                    } else {
                        "[No Image Available]"
                    };
                    Paragraph::new(image_text)
                        .alignment(Alignment::Center)
                        .block(image_block)
                };
                frame.render_widget(image_widget, chunks[0]);
            }

            // Enhanced preview panel with full episode information
            let mut info_lines: Vec<Line> = Vec::new();

            // Anime title - centered and prominent
            info_lines.push(Line::from(""));
            info_lines.push(Line::from(vec![Span::styled(
                history_title.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )]));
            info_lines.push(Line::from(""));

            // Show episode count info - fetch from provider if not cached
            let anime_id = format!(
                "{}:{}",
                history_provider,
                history_anime_id.split(':').nth(1).unwrap_or("")
            );
            let total_eps = self.continue_watching_metadata.get(&anime_id).copied();

            if let Some(total) = total_eps {
                info_lines.push(Line::from(vec![
                    Span::raw("Total Episodes: "),
                    Span::styled(total.to_string(), Style::default().fg(Color::Green)),
                ]));
            } else {
                // Show loading and trigger background fetch
                info_lines.push(Line::from(vec![
                    Span::raw("Total Episodes: "),
                    Span::styled("Loading...", Style::default().fg(Color::Yellow)),
                ]));
            }

            // Episode information with title if available
            let ep_display = if let Some(ref ep_title) = history_episode_title {
                format!("{} - {}", history_episode, ep_title)
            } else {
                history_episode.to_string()
            };

            info_lines.push(Line::from(vec![
                Span::raw("Current Episode: "),
                Span::styled(ep_display, Style::default().fg(Color::Green)),
            ]));

            // Provider
            info_lines.push(Line::from(vec![
                Span::raw("Provider: "),
                Span::styled(&history_provider, Style::default().fg(Color::Yellow)),
            ]));

            // Last watched timestamp
            let time_ago = Self::format_time_ago(history_updated);
            info_lines.push(Line::from(vec![
                Span::raw("Last watched: "),
                Span::styled(time_ago, Style::default().fg(Color::Magenta)),
            ]));

            // Progress bar if there's a saved position
            if history_position > 0 && history_total > 0 {
                let progress_pct = (history_position as f64 / history_total as f64 * 100.0) as u32;
                let progress_bar = Self::create_progress_bar(progress_pct, 20);
                let position_str = Self::format_duration(history_position);
                let total_str = Self::format_duration(history_total);

                info_lines.push(Line::from(""));
                info_lines.push(Line::from(vec![
                    Span::raw("Progress: "),
                    Span::styled(progress_bar, Style::default().fg(Color::Blue)),
                    Span::raw(format!(" {}%", progress_pct)),
                ]));
                info_lines.push(Line::from(vec![Span::raw(format!(
                    "{} / {}",
                    position_str, total_str
                ))]));
            }

            info_lines.push(Line::from(""));
            info_lines.push(Line::from(vec![Span::styled(
                "Press Enter to resume watching",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]));

            let info_widget = Paragraph::new(Text::from(info_lines))
                .block(Block::default().borders(Borders::ALL).title("Preview"));
            frame.render_widget(info_widget, chunks[1]);
        }
    }

    /// Format a duration in seconds to a human-readable string (HH:MM:SS or MM:SS)
    fn format_duration(seconds: u64) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, secs)
        } else {
            format!("{:02}:{:02}", minutes, secs)
        }
    }

    /// Format a DateTime to a "time ago" string (e.g., "2 hours ago", "3 days ago")
    fn format_time_ago(dt: chrono::DateTime<chrono::Utc>) -> String {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(dt);

        let seconds = duration.num_seconds();
        let minutes = duration.num_minutes();
        let hours = duration.num_hours();
        let days = duration.num_days();

        if seconds < 60 {
            "just now".to_string()
        } else if minutes < 60 {
            format!(
                "{} minute{} ago",
                minutes,
                if minutes == 1 { "" } else { "s" }
            )
        } else if hours < 24 {
            format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
        } else if days < 30 {
            format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
        } else {
            dt.format("%Y-%m-%d").to_string()
        }
    }

    /// Create a text progress bar
    fn create_progress_bar(percentage: u32, width: usize) -> String {
        let filled = (percentage as usize * width / 100).min(width);
        let empty = width - filled;
        format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
    }

    pub(crate) fn render_image_with_ratatui(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        image_data: &[u8],
    ) {
        if image_data.is_empty() {
            self.show_image_placeholder(frame, area, false);
            return;
        }

        if self.last_image_render.elapsed().as_millis() < 33 {
            let border_only = Block::default().borders(Borders::ALL).title("Cover Image");
            frame.render_widget(border_only, area);
            return;
        }

        let needs_update = match &self.current_image_data {
            Some(prev_data) => prev_data != image_data,
            None => true,
        };

        let border_only = Block::default().borders(Borders::ALL).title("Cover Image");
        let inner_area = border_only.inner(area);
        frame.render_widget(border_only, area);

        if needs_update {
            if self.image_renderer.requires_terminal_clear() {
                if let Err(e) = self.image_renderer.clear_terminal_graphics() {
                    tracing::warn!("Failed to clear terminal graphics: {}", e);
                }
            }
            self.image_renderer.clear_cache();
            self.current_image_data = Some(image_data.to_vec());
        }

        match self.image_renderer.render(image_data, inner_area) {
            Ok(_) => {
                tracing::debug!("Image rendered in inner area {:?}", inner_area);
                self.last_image_render = Instant::now();
            }
            Err(e) => {
                tracing::warn!("Image render error: {}", e);
                let error_lines = e.to_lines();
                let error_widget = Paragraph::new(ratatui::text::Text::from(error_lines))
                    .block(Block::default().borders(Borders::NONE));
                frame.render_widget(error_widget, inner_area);
            }
        }
    }

    fn show_image_placeholder(&self, frame: &mut Frame, area: Rect, is_error: bool) {
        let text = if is_error {
            "[Image error]"
        } else {
            "[No image]"
        };
        let placeholder = Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title("Cover Image"));
        frame.render_widget(placeholder, area);
    }

    fn draw_search(&mut self, frame: &mut Frame) {
        let area = frame.size();

        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(main_layout[0]);

        self.search_overlay.results = self.enriched_results.clone();
        self.search_overlay.selected_index = self.selected_index;
        self.search_overlay.is_searching =
            self.search_overlay.query.len() >= 2 && self.enriched_results.is_empty();
        self.search_overlay
            .render(frame, content_layout[0], &[self.selected_source]);

        let selected_anime = self.enriched_results.get(self.selected_index).cloned();
        PreviewPanel::render(frame, content_layout[1], selected_anime.as_ref(), self);

        let status_bar = Paragraph::new(
            "Shift+C: Change Source | ESC: Back Home | ↑/↓: Navigate | Enter: Select",
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
        frame.render_widget(status_bar, main_layout[1]);
    }

    fn draw_episode_select(&mut self, frame: &mut Frame) {
        let area = frame.size();

        // Get anime title and check for last watched episode
        let (title, last_watched_ep) = self
            .selected_anime
            .as_ref()
            .map(|a| {
                let anime_id = format!("{}:{}", a.base.provider, a.base.id);
                let last_ep = self
                    .continue_watching
                    .iter()
                    .find(|h| h.anime_id == anime_id)
                    .map(|h| h.episode_number);
                (a.base.title.clone(), last_ep)
            })
            .unwrap_or_else(|| ("Select Episode".to_string(), None));

        // Filter episodes based on search
        let filtered_episodes: Vec<(usize, &crate::providers::Episode)> = self
            .episodes
            .iter()
            .enumerate()
            .filter(|(_, ep)| {
                if self.episode_filter.is_empty() {
                    true
                } else {
                    let filter_lower = self.episode_filter.to_lowercase();
                    let ep_str = format!("{}", ep.number);
                    ep_str.contains(&filter_lower)
                        || ep
                            .title
                            .as_ref()
                            .map(|t| t.to_lowercase().contains(&filter_lower))
                            .unwrap_or(false)
                }
            })
            .collect();

        // Calculate pagination
        let total_episodes = filtered_episodes.len();
        let total_pages = (total_episodes + self.episodes_per_page - 1) / self.episodes_per_page;
        let current_page = self.episode_current_page.min(total_pages.saturating_sub(1));
        let page_start = current_page * self.episodes_per_page;
        let page_end = (page_start + self.episodes_per_page).min(total_episodes);
        let page_episodes: Vec<_> = filtered_episodes
            .iter()
            .skip(page_start)
            .take(self.episodes_per_page)
            .collect();

        // Split area into header, filter bar, grid, and footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(3), // Filter bar
                Constraint::Length(1), // Page info
                Constraint::Min(0),    // Episodes grid
                Constraint::Length(1), // Help text
            ])
            .split(area);

        // Header with anime title
        let header_text = if self.episode_filter_mode {
            format!("{} - Filter Episodes", title)
        } else {
            format!("{} - Select Episode to Watch", title)
        };
        let header = Paragraph::new(header_text)
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(header, chunks[0]);

        // Filter bar
        let filter_prompt = if self.episode_filter_mode { "> " } else { "  " };
        let filter_text = format!(
            "{}Filter: {}{}",
            filter_prompt,
            self.episode_filter,
            if self.episode_filter_mode { "_" } else { "" }
        );
        let filter_style = if self.episode_filter_mode {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        let filter_widget = Paragraph::new(filter_text)
            .style(filter_style)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(filter_widget, chunks[1]);

        // Page info
        let page_info = if total_pages > 1 {
            format!(
                "Page {} of {} (Episodes {}-{} of {})",
                current_page + 1,
                total_pages,
                page_start + 1,
                page_end,
                total_episodes
            )
        } else {
            format!(
                "Episodes {}-{} of {}",
                page_start + 1,
                page_end,
                total_episodes
            )
        };
        let page_widget = Paragraph::new(page_info)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(page_widget, chunks[2]);

        // Calculate grid dimensions for fullscreen layout
        let available_width = chunks[3].width as usize;
        let available_height = chunks[3].height as usize;
        let cell_width = 11usize; // Width of each episode cell [ XXXX ] with brackets and padding
        let cell_spacing = 6usize; // More space between cells
        let max_visible_rows = available_height.saturating_sub(2);

        // Dynamically calculate number of columns based on available width
        let cols = ((available_width - 4) / (cell_width + cell_spacing))
            .max(1)
            .min(10);

        // Build grid lines - center the grid with selected episode in the middle
        let mut grid_lines: Vec<Line> = Vec::new();
        let total_grid_width = cols * cell_width + (cols - 1) * cell_spacing;
        // Increase left padding to shift grid more to the right
        let left_padding = ((available_width.saturating_sub(total_grid_width)) / 2) + 15;

        let max_rows = max_visible_rows;
        let total_rows = (page_episodes.len() + cols - 1) / cols;
        let rows_to_show = total_rows.min(max_rows);

        // Calculate vertical padding to center the grid
        let vertical_padding = (max_rows.saturating_sub(rows_to_show)) / 2;

        // Add top padding
        for _ in 0..vertical_padding {
            grid_lines.push(Line::from(""));
        }

        // Render all episodes in the current page
        for row in 0..rows_to_show {
            let mut row_spans: Vec<Span> = Vec::new();

            // Add left padding to center the grid horizontally
            if left_padding > 0 {
                row_spans.push(Span::raw(" ".repeat(left_padding)));
            }

            for col in 0..cols {
                let idx = row * cols + col;

                if let Some((ep_idx, episode)) = page_episodes.get(idx) {
                    let is_selected = *ep_idx == self.episode_selected_index;
                    let is_last_watched = last_watched_ep
                        .map(|ep| ep == episode.number)
                        .unwrap_or(false);

                    // Format episode number with padding for up to 4 digits (e.g., 1156)
                    let ep_display = format!(" {:>4} ", episode.number);

                    // Determine style based on state with better visibility
                    let (bg_color, fg_color) = if is_selected {
                        (Color::Yellow, Color::Black)
                    } else if is_last_watched {
                        (Color::Red, Color::White)
                    } else {
                        (Color::DarkGray, Color::White)
                    };

                    // Create cell with proper styling
                    let cell_style = Style::default()
                        .fg(fg_color)
                        .bg(bg_color)
                        .add_modifier(Modifier::BOLD);

                    // Add spacing between cells
                    if col > 0 {
                        row_spans.push(Span::raw(" ".repeat(cell_spacing)));
                    }

                    row_spans.push(Span::styled(format!("[{}]", ep_display), cell_style));
                } else {
                    // Empty cell
                    if col > 0 {
                        row_spans.push(Span::raw(" ".repeat(cell_spacing)));
                    }
                    row_spans.push(Span::raw(" ".repeat(cell_width)));
                }
            }

            grid_lines.push(Line::from(row_spans));
        }

        // Add bottom padding to fill space
        while grid_lines.len() < max_rows {
            grid_lines.push(Line::from(""));
        }

        let grid_widget =
            Paragraph::new(Text::from(grid_lines)).block(Block::default().borders(Borders::ALL));
        frame.render_widget(grid_widget, chunks[3]);

        // Help text
        let help_text = if self.episode_filter_mode {
            "Type to filter | Esc: Exit filter | Enter: Play"
        } else if total_pages > 1 {
            "↑↓: Move Up/Down | ←→: Move Left/Right | PgUp/PgDn: Change Page | /: Filter | Enter: Play | Esc: Back"
        } else {
            "↑↓: Move Up/Down | ←→: Move Left/Right | /: Filter | Enter: Play | Esc: Back"
        };
        let help_widget = Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(help_widget, chunks[4]);
    }

    fn draw_player(&mut self, frame: &mut Frame) {
        match self.player_controller.state() {
            PlayerState::Playing | PlayerState::ControlsVisible => {
                self.draw_player_with_controls(frame);
            }
            PlayerState::Ended => {
                self.draw_end_screen(frame);
            }
        }
    }

    fn draw_player_with_controls(&mut self, frame: &mut Frame) {
        let area = frame.size();

        // Always show control overlay (no intermediate "Video playing..." screen)
        self.draw_control_overlay(frame, area);

        // Episode list modal
        if self.show_episode_list {
            self.draw_episode_list_modal(frame);
        }
    }

    fn draw_control_overlay(&self, frame: &mut Frame, area: Rect) {
        // Control items: Previous, Next, Choose, Back
        // Order: 0=Previous, 1=Next, 2=Choose, 3=Back
        let controls = vec![
            (
                "Previous Episode",
                "[P]",
                self.player_controller.has_previous_episode(),
            ),
            (
                "Next Episode",
                "[N]",
                self.player_controller.has_next_episode(),
            ),
            ("Choose Episode", "[E]", true),
            ("Back to Dashboard", "[ESC]", true),
        ];

        let title = format!(
            "{} - Episode {}/{}",
            self.player_controller.anime_title().unwrap_or("Unknown"),
            self.player_controller.episode_number(),
            self.player_controller.total_episodes()
        );

        let available_height = area.height as usize;
        let vertical_padding = (available_height.saturating_sub(10)) / 2;

        let mut all_lines: Vec<Line> = Vec::new();

        // Add top padding
        for _ in 0..vertical_padding {
            all_lines.push(Line::from(""));
        }

        // Add title - larger and bold
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(vec![Span::styled(
            title.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )]));
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(""));

        // Build horizontal controls line with centered Next button
        let selected_idx = self.player_controller.selected_control();
        let mut control_spans: Vec<Span> = Vec::new();

        for (idx, (label, keybind, enabled)) in controls.iter().enumerate() {
            let is_selected = idx == selected_idx;

            let label_style = if !enabled {
                Style::default().fg(Color::DarkGray)
            } else if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            };

            let keybind_style = Style::default().fg(Color::Gray);

            // Add spacing between controls
            if idx > 0 {
                control_spans.push(Span::raw("        ")); // 8 spaces between items
            }

            // Add selection arrow if selected
            if is_selected {
                control_spans.push(Span::styled("▶ ", label_style));
            } else {
                control_spans.push(Span::raw("  "));
            }

            // Add keybind
            control_spans.push(Span::styled(keybind.to_string(), keybind_style));
            control_spans.push(Span::raw(" "));

            // Add label
            control_spans.push(Span::styled(label.to_string(), label_style));
        }

        all_lines.push(Line::from(control_spans));
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(""));

        // Add bottom caption with navigation help
        all_lines.push(Line::from(vec![Span::styled(
            "← → or ↑ ↓ : Navigate    Enter : Select",
            Style::default().fg(Color::Gray),
        )]));

        // Add remaining padding
        while all_lines.len() < available_height.saturating_sub(1) {
            all_lines.push(Line::from(""));
        }

        // No border block - clean minimal design
        let paragraph = Paragraph::new(Text::from(all_lines)).alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    fn draw_episode_list_modal(&mut self, frame: &mut Frame) {
        let area = centered_rect(60, 70, frame.size());

        frame.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Choose Episode");

        let lines = crate::ui::player_controller::EpisodeListModal::render(
            &self.episodes,
            self.player_controller.current_episode_idx(),
            self.episode_list_selected,
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        );

        let paragraph = Paragraph::new(Text::from(lines)).block(block);
        frame.render_widget(paragraph, area);
    }

    fn draw_end_screen(&mut self, frame: &mut Frame) {
        let area = frame.size();

        let options = crate::ui::player_controller::EndScreen::render(
            self.player_controller.has_next_episode(),
        );

        let mut lines: Vec<Line> = vec![
            Line::from(vec![Span::styled(
                "Video Ended\n",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
        ];

        for (line, _) in options {
            lines.push(line);
        }

        lines.push(Line::from(""));
        lines.push(Line::from("Press Enter to select, Esc to go back"));

        let paragraph = Paragraph::new(Text::from(lines))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Finished"));

        frame.render_widget(paragraph, area);
    }

    fn draw_source_modal(&mut self, frame: &mut Frame) {
        let frame_area = frame.size();

        // Create a smaller, centered modal area (40% width, 30% height)
        let modal_width = ((frame_area.width as f32 * 0.4).min(50.0).max(35.0)) as u16;
        let modal_height = 12u16; // Fixed height for 2 sources + header + footer

        let modal_area = Rect {
            x: frame_area.x + (frame_area.width.saturating_sub(modal_width)) / 2,
            y: frame_area.y + (frame_area.height.saturating_sub(modal_height)) / 2,
            width: modal_width,
            height: modal_height,
        };

        frame.render_widget(Clear, modal_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Select Source (Enter to confirm)");

        let inner_area = block.inner(modal_area);
        let chunks = Layout::default()
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
        frame.render_widget(caption, chunks[0]);

        let sources: Vec<(String, Language, bool)> = vec![
            (
                "AllAnime (English)".to_string(),
                Language::English,
                self.selected_source == Language::English,
            ),
            (
                "KKPhim (Vietnamese)".to_string(),
                Language::Vietnamese,
                self.selected_source == Language::Vietnamese,
            ),
        ];

        let mut lines: Vec<Line> = Vec::new();
        for (idx, (name, lang, enabled)) in sources.iter().enumerate() {
            let is_selected = idx == self.selected_source_idx;
            let prefix = if is_selected { "▶ " } else { "  " };
            let radio = if *enabled { "(◉)" } else { "(○)" };

            let lang_badge = match lang {
                Language::English => Span::styled("[EN]", Style::default().fg(Color::Blue)),
                Language::Vietnamese => Span::styled("[VN]", Style::default().fg(Color::Yellow)),
            };

            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            lines.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled(radio.to_string(), style),
                Span::raw(" "),
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
        frame.render_widget(paragraph, chunks[1]);
        frame.render_widget(block, modal_area);
    }

    fn draw_toast(&self, frame: &mut Frame, toast: &Toast) {
        let area = frame.size();
        let toast_area = Rect {
            x: area.width / 4,
            y: area.height - 5,
            width: area.width / 2,
            height: 3,
        };

        let paragraph = Paragraph::new(toast.message.clone())
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(paragraph, toast_area);
    }

    async fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        if self.show_source_modal {
            self.handle_source_modal_key(key).await?;
            return Ok(());
        }

        if self.show_episode_list {
            self.handle_episode_list_key(key).await?;
            return Ok(());
        }

        match self.current_screen {
            Screen::Splash => self.handle_splash_key(key).await,
            Screen::SourceSelect => self.handle_source_select_key(key).await,
            Screen::Home => self.handle_home_key(key).await,
            Screen::Search => self.handle_search_key(key).await,
            Screen::EpisodeSelect => self.handle_episode_select_key(key).await,
            Screen::Player => self.handle_player_key(key).await,
        }
    }

    async fn handle_episode_select_key(&mut self, key: KeyCode) -> Result<()> {
        let total_episodes = self.episodes.len();

        // Handle filter mode
        if self.episode_filter_mode {
            match key {
                KeyCode::Esc => {
                    self.episode_filter_mode = false;
                    self.episode_filter.clear();
                    // Reset to first episode when exiting filter
                    self.episode_selected_index = 0;
                }
                KeyCode::Backspace => {
                    self.episode_filter.pop();
                }
                KeyCode::Char(c) => {
                    // Only accept numeric input for episode filtering
                    if c.is_numeric() {
                        self.episode_filter.push(c);
                    }
                }
                KeyCode::Enter => {
                    // Try to jump to episode number
                    if let Ok(ep_num) = self.episode_filter.parse::<u32>() {
                        // Find episode with matching number
                        if let Some((idx, _)) = self
                            .episodes
                            .iter()
                            .enumerate()
                            .find(|(_, ep)| ep.number == ep_num)
                        {
                            self.episode_selected_index = idx;
                            // Update page to show selected episode
                            self.episode_current_page = idx / self.episodes_per_page;
                        }
                    }
                    self.episode_filter_mode = false;
                    self.episode_filter.clear();
                }
                _ => {}
            }
            return Ok(());
        }

        // Normal navigation mode with pagination
        let total_pages = (total_episodes + self.episodes_per_page - 1) / self.episodes_per_page;

        // Calculate number of columns based on terminal width (same as draw function)
        let available_width = 76usize;
        let cell_width = 9usize;
        let cell_spacing = 3usize;
        let cols = ((available_width - 4) / (cell_width + cell_spacing))
            .max(1)
            .min(10);

        match key {
            KeyCode::Esc | KeyCode::Char('b') => {
                let target = self.previous_screen.take().unwrap_or(Screen::Home);
                if target == Screen::Player {
                    self.player_controller.show_controls();
                }
                self.current_screen = target;
                if target != Screen::Player {
                    self.episodes.clear();
                }
                self.episode_selected_index = 0;
                self.episode_current_page = 0;
                self.episode_filter.clear();
                self.episode_filter_mode = false;
                // Trigger preview reload when returning to Search or Home
                if target == Screen::Search {
                    self.needs_preview_load = true;
                } else if target == Screen::Home {
                    self.needs_continue_watching_refresh = true;
                }
            }
            KeyCode::Up => {
                // Move up one row (subtract columns)
                if self.episode_selected_index >= cols {
                    self.episode_selected_index -= cols;
                    // Update page to show selected episode
                    self.episode_current_page =
                        self.episode_selected_index / self.episodes_per_page;
                }
            }
            KeyCode::Down => {
                // Move down one row (add columns)
                if self.episode_selected_index + cols < total_episodes {
                    self.episode_selected_index += cols;
                    // Update page to show selected episode
                    self.episode_current_page =
                        self.episode_selected_index / self.episodes_per_page;
                }
            }
            KeyCode::Left => {
                // Move left one column
                if self.episode_selected_index > 0 {
                    self.episode_selected_index -= 1;
                    // Update page to show selected episode
                    self.episode_current_page =
                        self.episode_selected_index / self.episodes_per_page;
                }
            }
            KeyCode::Right => {
                // Move right one column
                if self.episode_selected_index < total_episodes.saturating_sub(1) {
                    self.episode_selected_index += 1;
                    // Update page to show selected episode
                    self.episode_current_page =
                        self.episode_selected_index / self.episodes_per_page;
                }
            }
            KeyCode::PageUp => {
                // Go to previous page
                if self.episode_current_page > 0 {
                    self.episode_current_page -= 1;
                    // Set selection to first episode of new page
                    self.episode_selected_index =
                        self.episode_current_page * self.episodes_per_page;
                }
            }
            KeyCode::PageDown => {
                // Go to next page
                if self.episode_current_page + 1 < total_pages {
                    self.episode_current_page += 1;
                    // Set selection to first episode of new page
                    self.episode_selected_index =
                        self.episode_current_page * self.episodes_per_page;
                }
            }
            KeyCode::Home => {
                // Go to first episode
                self.episode_selected_index = 0;
                self.episode_current_page = 0;
            }
            KeyCode::End => {
                // Go to last episode
                self.episode_selected_index = total_episodes.saturating_sub(1);
                self.episode_current_page = (total_episodes - 1) / self.episodes_per_page;
            }
            KeyCode::Char('/') => {
                self.episode_filter_mode = true;
                self.episode_filter.clear();
            }
            KeyCode::Char(c) if c.is_numeric() => {
                // Quick jump: type episode number and press Enter
                self.episode_filter.push(c);
                self.episode_filter_mode = true;
            }
            KeyCode::Enter => {
                if self.episode_selected_index < total_episodes {
                    let anime = self
                        .selected_anime
                        .as_ref()
                        .map(|a| a.base.clone())
                        .or_else(|| self.player_controller.current_anime().cloned());

                    if let Some(anime) = anime {
                        self.player_controller.start_playback(
                            anime,
                            self.episodes.clone(),
                            self.episode_selected_index,
                        );
                        self.current_screen = Screen::Player;
                        self.play_current_episode().await;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_splash_key(&mut self, key: KeyCode) -> Result<()> {
        if key == KeyCode::Enter || key == KeyCode::Esc {
            self.current_screen = Screen::Home;
        }
        Ok(())
    }

    async fn handle_source_select_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.current_screen = Screen::Home;
            }
            KeyCode::Up => {
                if self.selected_source_idx > 0 {
                    self.selected_source_idx -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_source_idx < 1 {
                    self.selected_source_idx += 1;
                }
            }
            KeyCode::Enter => {
                let new_source = if self.selected_source_idx == 0 {
                    Language::English
                } else {
                    Language::Vietnamese
                };
                if new_source != self.selected_source {
                    tracing::info!(
                        "Source changed from {:?} to {:?}",
                        self.selected_source,
                        new_source
                    );
                    self.selected_source = new_source;
                }
                // Clear any existing images before transitioning
                if self.image_renderer.requires_terminal_clear() {
                    let _ = self.image_renderer.clear_terminal_graphics();
                }
                self.current_image_data = None;
                self.current_anime_id = None;
                self.previous_image_data = None;
                self.in_transition = false;
                self.image_renderer.clear_cache();
                self.last_preview_anime_id = None;
                // After selecting source, go to Search screen
                self.current_screen = Screen::Search;
                self.needs_preview_load = true;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_home_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
            }
            KeyCode::Up => {
                if self.continue_watching_selected > 0 {
                    self.continue_watching_selected -= 1;
                    self.load_continue_watching_image().await;
                }
            }
            KeyCode::Down => {
                let max_idx = self.continue_watching.len().saturating_sub(1);
                if self.continue_watching_selected < max_idx {
                    self.continue_watching_selected += 1;
                    self.load_continue_watching_image().await;
                }
            }
            KeyCode::Enter => {
                // Resume the selected anime
                if let Some(history) = self
                    .continue_watching
                    .get(self.continue_watching_selected)
                    .cloned()
                {
                    self.resume_watching(history).await?;
                }
            }
            KeyCode::Char('D') => {
                // Remove from continue watching
                if !self.continue_watching.is_empty()
                    && self.continue_watching_selected < self.continue_watching.len()
                {
                    let history = &self.continue_watching[self.continue_watching_selected];
                    let anime_id = history.anime_id.clone();
                    let title = history.title.clone();

                    // Remove from database
                    let _ = self.db.remove_from_continue_watching(&anime_id).await;

                    // Remove from local list
                    self.continue_watching
                        .remove(self.continue_watching_selected);

                    // Adjust selection
                    if self.continue_watching_selected >= self.continue_watching.len()
                        && self.continue_watching_selected > 0
                    {
                        self.continue_watching_selected -= 1;
                    }

                    self.show_toast(format!("Removed '{}' from Continue Watching", title), 2);
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn resume_watching(&mut self, history: crate::db::WatchHistory) -> Result<()> {
        tracing::info!(
            "Resuming anime: {} - Ep {}",
            history.title,
            history.episode_number
        );

        // Find the provider for this history entry
        if let Some(_provider) = self.providers.get_provider(&history.provider) {
            // Create a basic Anime struct from history
            let anime = crate::providers::Anime {
                id: history
                    .anime_id
                    .split(':')
                    .nth(1)
                    .unwrap_or(&history.anime_id)
                    .to_string(),
                provider: history.provider.clone(),
                title: history.title.clone(),
                cover_url: history.cover_url.clone(),
                language: if history.provider == "KKPhim" {
                    crate::providers::Language::Vietnamese
                } else {
                    crate::providers::Language::English
                },
                total_episodes: None,
                synopsis: None,
            };

            self.select_anime(anime).await?;
        } else {
            self.show_toast(format!("Provider {} not available", history.provider), 3);
        }

        Ok(())
    }

    async fn load_continue_watching_image(&mut self) {
        // Debounce: skip if we updated recently (100ms for better responsiveness)
        if self.last_image_navigation.elapsed().as_millis() < 100 {
            return;
        }
        self.last_image_navigation = Instant::now();

        if let Some(history) = self.continue_watching.get(self.continue_watching_selected) {
            // Check if this is a different anime than currently displayed
            let is_new_anime = self.current_anime_id.as_ref() != Some(&history.anime_id);

            // Check preloaded cache first for instant display
            if let Some(preloaded_data) = self.preloaded_images.get(&history.anime_id) {
                tracing::debug!("Using preloaded image for: {}", history.title);

                // Store previous image for transition
                if is_new_anime && self.current_image_data.is_some() {
                    self.previous_image_data = self.current_image_data.clone();
                    self.in_transition = true;
                    self.transition_progress = 0.0;
                }

                self.current_image_data = Some(preloaded_data.clone());
                self.current_anime_id = Some(history.anime_id.clone());
                self.last_image_render = Instant::now();
                self.image_loading = false;

                // Preload next/prev images after displaying current
                self.preload_adjacent_images().await;
                return;
            }

            // Set loading state before async load
            if is_new_anime {
                self.image_loading = true;
            }

            if !history.cover_url.is_empty() {
                let image_id = format!("continue_watching_{}", history.anime_id);
                let cover_url = history.cover_url.clone();

                // Load image - request_download checks memory -> disk -> network
                match self
                    .image_pipeline
                    .request_download(image_id, cover_url)
                    .await
                {
                    Ok(data) => {
                        // Store previous image for transition
                        if is_new_anime && self.current_image_data.is_some() {
                            self.previous_image_data = self.current_image_data.clone();
                            self.in_transition = true;
                            self.transition_progress = 0.0;
                        }

                        // Update current image
                        self.current_image_data = Some(data.clone());
                        self.current_anime_id = Some(history.anime_id.clone());
                        self.last_image_render = Instant::now();
                        self.image_loading = false;

                        // Cache for preloading
                        self.preloaded_images.insert(history.anime_id.clone(), data);

                        tracing::debug!("Loaded continue watching image for: {}", history.title);

                        // Preload next/prev images
                        self.preload_adjacent_images().await;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load cover image for {}: {}", history.title, e);
                        // Show placeholder on error
                        self.current_image_data = None;
                        self.current_anime_id = Some(history.anime_id.clone());
                        self.image_loading = false;
                    }
                }
            } else {
                self.current_image_data = None;
                self.current_anime_id = Some(history.anime_id.clone());
            }
        }
    }

    /// Preload next and previous anime images for smooth navigation
    async fn preload_adjacent_images(&mut self) {
        let current_idx = self.continue_watching_selected;

        // Preload next anime
        if let Some(next_history) = self.continue_watching.get(current_idx + 1) {
            if !next_history.cover_url.is_empty()
                && !self.preloaded_images.contains_key(&next_history.anime_id)
            {
                let image_id = format!("continue_watching_{}", next_history.anime_id);
                let cover_url = next_history.cover_url.clone();
                let anime_id = next_history.anime_id.clone();

                // Spawn background task to preload
                if let Ok(data) = self
                    .image_pipeline
                    .request_download(image_id, cover_url)
                    .await
                {
                    self.preloaded_images.insert(anime_id, data);
                    tracing::debug!("Preloaded next image for: {}", next_history.title);
                }
            }
        }

        // Preload previous anime
        if current_idx > 0 {
            if let Some(prev_history) = self.continue_watching.get(current_idx - 1) {
                if !prev_history.cover_url.is_empty()
                    && !self.preloaded_images.contains_key(&prev_history.anime_id)
                {
                    let image_id = format!("continue_watching_{}", prev_history.anime_id);
                    let cover_url = prev_history.cover_url.clone();
                    let anime_id = prev_history.anime_id.clone();

                    if let Ok(data) = self
                        .image_pipeline
                        .request_download(image_id, cover_url)
                        .await
                    {
                        self.preloaded_images.insert(anime_id, data);
                        tracing::debug!("Preloaded previous image for: {}", prev_history.title);
                    }
                }
            }
        }

        // Limit cache size to prevent memory bloat (keep last 10 images)
        if self.preloaded_images.len() > 10 {
            // Remove oldest entries (simple approach: clear all except current and adjacent)
            let current_id = self.current_anime_id.clone();
            let next_id = self
                .continue_watching
                .get(current_idx + 1)
                .map(|h| h.anime_id.clone());
            let prev_id = if current_idx > 0 {
                self.continue_watching
                    .get(current_idx - 1)
                    .map(|h| h.anime_id.clone())
            } else {
                None
            };

            self.preloaded_images.retain(|id, _| {
                Some(id.clone()) == current_id
                    || Some(id.clone()) == next_id
                    || Some(id.clone()) == prev_id
            });
        }
    }

    /// Load metadata in background after splash screen
    async fn load_metadata_in_background(&mut self) {
        // This runs in background, doesn't block UI
        let history_items: Vec<_> = self.continue_watching.clone();

        for history in &history_items {
            let parts: Vec<&str> = history.anime_id.split(':').collect();
            if parts.len() >= 2 {
                let provider = parts[0];
                let anime_id = parts[1];

                // Get episode count from provider directly
                if let Some(provider_obj) = self.providers.get_provider(provider) {
                    match provider_obj.get_episodes(anime_id).await {
                        Ok(episodes) => {
                            let key = format!("{}:{}", provider, anime_id);
                            self.continue_watching_metadata
                                .insert(key, episodes.len() as u32);
                            tracing::debug!(
                                "Loaded {} episodes for {}",
                                episodes.len(),
                                history.title
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load episodes for {}: {}", history.title, e);
                        }
                    }
                }
            }
        }

        tracing::info!(
            "Background metadata loading complete for {} items",
            self.continue_watching_metadata.len()
        );
    }

    async fn handle_search_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Char('B') => {
                // Clear terminal graphics if using Kitty protocol
                if self.image_renderer.requires_terminal_clear() {
                    let _ = self.image_renderer.clear_terminal_graphics();
                }
                self.current_screen = Screen::Home;
                self.search_overlay.query.clear();
                self.enriched_results.clear();
                self.search_pending = false;
                // Clear terminal graphics for all protocols
                let _ = self.image_renderer.clear_terminal_graphics();
                // Clear all image state
                self.current_image_data = None;
                self.current_anime_id = None;
                self.previous_image_data = None;
                self.in_transition = false;
                self.image_renderer.clear_cache();
                self.last_preview_anime_id = None;
                self.last_keypress = Instant::now();
            }
            KeyCode::Backspace => {
                self.search_overlay.query.pop();
                self.search_pending = true;
                self.last_keypress = Instant::now();
                // Clear results and image if empty
                if self.search_overlay.query.is_empty() {
                    self.enriched_results.clear();
                    self.search_pending = false;
                    // Clear image when search query is empty
                    if self.image_renderer.requires_terminal_clear() {
                        let _ = self.image_renderer.clear_terminal_graphics();
                    }
                    // Clear all image state
                    self.current_image_data = None;
                    self.current_anime_id = None;
                    self.previous_image_data = None;
                    self.in_transition = false;
                    self.image_renderer.clear_cache();
                    self.last_preview_anime_id = None;
                }
            }
            KeyCode::Char(c) => {
                self.search_overlay.query.push(c);
                self.search_pending = true;
                self.last_keypress = Instant::now();
                // Clear pending search if query is too short
                if self.search_overlay.query.len() < 2 {
                    self.search_pending = false;
                }
            }
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.load_preview().await;
                }
            }
            KeyCode::Down => {
                if self.selected_index < self.enriched_results.len().saturating_sub(1) {
                    self.selected_index += 1;
                    self.load_preview().await;
                }
            }
            KeyCode::Enter => {
                if let Some(anime) = self.enriched_results.get(self.selected_index).cloned() {
                    self.selected_anime = Some(anime.clone());
                    self.select_anime(anime.base).await?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_episode_list_key(&mut self, key: KeyCode) -> Result<()> {
        let total_episodes = self.episodes.len();
        let available_width = 60usize;
        let cell_width = 11usize;
        let cell_spacing = 4usize;
        let cols = ((available_width.saturating_sub(4)) / (cell_width + cell_spacing)).max(1);

        match key {
            KeyCode::Esc => {
                self.show_episode_list = false;
            }
            KeyCode::Up => {
                if self.episode_list_selected >= cols {
                    self.episode_list_selected -= cols;
                }
            }
            KeyCode::Down => {
                if self.episode_list_selected + cols < total_episodes {
                    self.episode_list_selected += cols;
                }
            }
            KeyCode::Left => {
                if self.episode_list_selected > 0 {
                    self.episode_list_selected -= 1;
                }
            }
            KeyCode::Right => {
                if self.episode_list_selected < total_episodes.saturating_sub(1) {
                    self.episode_list_selected += 1;
                }
            }
            KeyCode::Enter => {
                if self.episode_list_selected < total_episodes {
                    self.player_controller
                        .select_episode(self.episode_list_selected);
                    self.play_current_episode().await;
                }
                self.show_episode_list = false;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_player_key(&mut self, key: KeyCode) -> Result<()> {
        match self.player_controller.state() {
            PlayerState::Playing => {
                // Any key shows controls (should rarely happen now since we start with ControlsVisible)
                self.player_controller.show_controls();
            }
            PlayerState::ControlsVisible => {
                match key {
                    KeyCode::Esc => {
                        // ESC goes back to Dashboard
                        self.save_watch_history().await;
                        self.current_screen = Screen::Home;
                        self.player_controller = PlayerController::new();
                    }
                    KeyCode::Up | KeyCode::Left => {
                        // Up or Left goes to previous control
                        self.player_controller.previous_control();
                    }
                    KeyCode::Down | KeyCode::Right => {
                        // Down or Right goes to next control
                        self.player_controller.next_control();
                    }
                    KeyCode::Enter => {
                        self.execute_control_action().await;
                    }
                    KeyCode::Char('n') => {
                        if self.player_controller.play_next_episode() {
                            self.play_current_episode().await;
                        }
                    }
                    KeyCode::Char('p') => {
                        if self.player_controller.play_previous_episode() {
                            self.play_current_episode().await;
                        }
                    }
                    KeyCode::Char('e') => {
                        self.previous_screen = Some(Screen::Player);
                        self.episode_selected_index = self.player_controller.current_episode_idx();
                        self.episode_current_page =
                            self.episode_selected_index / self.episodes_per_page;

                        if self.episodes.is_empty() {
                            if let Some((anime, episodes)) =
                                self.player_controller.current_anime_and_episodes()
                            {
                                self.selected_anime = Some(crate::metadata::EnrichedAnime {
                                    base: anime,
                                    metadata: None,
                                });
                                self.episodes = episodes;
                            }
                        }

                        self.current_screen = Screen::EpisodeSelect;
                    }
                    _ => {}
                }
            }
            PlayerState::Ended => {
                match key {
                    KeyCode::Esc => {
                        // ESC goes back to Dashboard
                        self.save_watch_history().await;
                        self.current_screen = Screen::Home;
                        self.player_controller = PlayerController::new();
                    }
                    KeyCode::Enter => {
                        // Default action: next episode or back
                        if self.player_controller.has_next_episode() {
                            self.player_controller.play_next_episode();
                            self.play_current_episode().await;
                        } else {
                            self.current_screen = Screen::Home;
                            self.player_controller = PlayerController::new();
                        }
                    }
                    KeyCode::Char('r') => {
                        self.play_current_episode().await;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    async fn handle_source_modal_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.source_modal_for_search = false;
                self.show_source_modal = false;
            }
            KeyCode::Up => {
                if self.selected_source_idx > 0 {
                    self.selected_source_idx -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_source_idx < 1 {
                    self.selected_source_idx += 1;
                }
            }
            KeyCode::Enter => {
                let new_source = if self.selected_source_idx == 0 {
                    Language::English
                } else {
                    Language::Vietnamese
                };
                if new_source != self.selected_source {
                    tracing::info!(
                        "Source changed from {:?} to {:?}",
                        self.selected_source,
                        new_source
                    );
                    self.selected_source = new_source;
                }
                self.show_source_modal = false;
                self.source_modal_for_search = false;

                // If on Search screen, refresh results
                if self.current_screen == Screen::Search && !self.search_overlay.query.is_empty() {
                    self.perform_search().await;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn execute_control_action(&mut self) {
        match self.player_controller.get_selected_action() {
            ControlAction::NextEpisode => {
                if self.player_controller.play_next_episode() {
                    self.play_current_episode().await;
                }
            }
            ControlAction::PreviousEpisode => {
                if self.player_controller.play_previous_episode() {
                    self.play_current_episode().await;
                }
            }
            ControlAction::ChooseEpisode => {
                self.previous_screen = Some(Screen::Player);
                self.episode_selected_index = self.player_controller.current_episode_idx();
                self.episode_current_page = self.episode_selected_index / self.episodes_per_page;

                if self.episodes.is_empty() {
                    if let Some((anime, episodes)) =
                        self.player_controller.current_anime_and_episodes()
                    {
                        self.selected_anime = Some(crate::metadata::EnrichedAnime {
                            base: anime,
                            metadata: None,
                        });
                        self.episodes = episodes;
                    }
                }

                self.current_screen = Screen::EpisodeSelect;
            }
            ControlAction::BackToMenu => {
                self.current_screen = Screen::Home;
                self.player_controller = PlayerController::new();
            }
        }
    }

    async fn perform_search(&mut self) {
        let query = self.search_overlay.query.clone();

        // Don't search if query is too short
        if query.len() < 2 {
            self.enriched_results.clear();
            return;
        }

        tracing::info!(
            "Searching for '{}' with source: {:?}",
            query,
            self.selected_source
        );

        // Search selected source only
        let mut all_results = Vec::new();

        match self
            .providers
            .search_filtered(&query, &[self.selected_source])
            .await
        {
            Ok(mut results) => {
                tracing::info!(
                    "Found {} results from {:?}",
                    results.len(),
                    self.selected_source
                );
                all_results.append(&mut results);
            }
            Err(e) => {
                tracing::warn!("Search failed for {:?}: {}", self.selected_source, e);
            }
        }

        // Update results
        let enriched: Vec<_> = all_results
            .into_iter()
            .map(|base| crate::metadata::EnrichedAnime {
                base,
                metadata: None,
            })
            .collect();

        self.enriched_results = enriched;
        self.selected_index = 0;

        // Load preview for first result
        if !self.enriched_results.is_empty() {
            self.load_preview().await;
        }
    }

    async fn load_preview(&mut self) {
        // Debounce: skip if we updated recently (100ms for better responsiveness)
        if self.last_image_navigation.elapsed().as_millis() < 100 {
            return;
        }
        self.last_image_navigation = Instant::now();

        if let Some(anime) = self.enriched_results.get_mut(self.selected_index) {
            // Check if this is a different anime than currently displayed
            let is_new_anime = self.current_anime_id.as_ref() != Some(&anime.base.id);

            // Load image - use request_download which checks memory -> disk -> network
            let id = anime.base.id.clone();
            let url = anime.base.cover_url.clone();

            if !url.is_empty() {
                match self.image_pipeline.request_download(id, url).await {
                    Ok(data) => {
                        // Store previous image for transition
                        if is_new_anime && self.current_image_data.is_some() {
                            self.previous_image_data = self.current_image_data.clone();
                            self.in_transition = true;
                            self.transition_progress = 0.0;
                        }

                        // Update current image
                        self.current_image_data = Some(data);
                        self.current_anime_id = Some(anime.base.id.clone());
                        self.last_image_render = Instant::now();
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load preview image: {}", e);
                        // Show placeholder on error
                        self.current_image_data = None;
                        self.current_anime_id = Some(anime.base.id.clone());
                    }
                }
            } else {
                self.current_image_data = None;
                self.current_anime_id = Some(anime.base.id.clone());
            }

            // Fetch metadata if not already loaded
            if anime.metadata.is_none() {
                if let Ok(metadata_list) = self
                    .metadata_cache
                    .search_and_cache(&anime.base.title)
                    .await
                {
                    // Use first metadata result
                    if let Some(metadata) = metadata_list.into_iter().next() {
                        anime.metadata = Some(metadata);
                    }
                }
            }
        }
    }

    async fn save_watch_history(&self) {
        use chrono::Utc;

        if let Some(anime) = self.selected_anime.as_ref() {
            if let Some(episode) = self.player_controller.current_episode() {
                let anime_id = format!("{}:{}", anime.base.provider, anime.base.id);
                let history = crate::db::WatchHistory {
                    anime_id,
                    provider: anime.base.provider.clone(),
                    title: anime.base.title.clone(),
                    cover_url: anime.base.cover_url.clone(),
                    episode_number: episode.number,
                    episode_title: episode.title.clone(),
                    position_seconds: 0, // We don't track exact position yet
                    total_seconds: 0,    // We don't know total duration
                    updated_at: Utc::now(),
                };
                let _ = self.db.save_watch_history(&history).await;
                tracing::info!(
                    "Saved watch history for {} ep {}",
                    anime.base.title,
                    episode.number
                );
            }
        }
    }

    async fn select_anime(&mut self, anime: crate::providers::Anime) -> Result<()> {
        tracing::info!(
            "Selecting anime: {} from provider: {}",
            anime.title,
            anime.provider
        );

        self.selected_anime = Some(crate::metadata::EnrichedAnime {
            base: anime.clone(),
            metadata: None,
        });
        self.episodes.clear();
        self.show_toast(format!("Loading episodes for {}...", anime.title), 3);

        // Find last watched episode position for this anime
        let anime_id = format!("{}:{}", anime.provider, anime.id);
        let last_watched_ep = self
            .continue_watching
            .iter()
            .find(|h| h.anime_id == anime_id)
            .map(|h| h.episode_number);

        // Load episodes from the provider
        if let Some(provider) = self.providers.get_provider(&anime.provider) {
            tracing::info!(
                "Found provider, loading episodes for anime_id: {}",
                anime.id
            );
            match provider.get_episodes(&anime.id).await {
                Ok(episodes) => {
                    tracing::info!("Loaded {} episodes", episodes.len());
                    self.episodes = episodes;
                    if !self.episodes.is_empty() {
                        // Set selection to last watched episode, or 0 if not found
                        self.episode_selected_index = last_watched_ep
                            .and_then(|ep| self.episodes.iter().position(|e| e.number == ep))
                            .unwrap_or(0);
                        // Calculate which page the selected episode is on
                        self.episode_current_page =
                            self.episode_selected_index / self.episodes_per_page;
                        // Also set legacy scroll for compatibility
                        self.episode_list_scroll = self.episode_selected_index;
                        // Reset filter state
                        self.episode_filter.clear();
                        self.episode_filter_mode = false;
                        // Save current screen for back navigation
                        self.previous_screen = Some(self.current_screen);
                        self.current_screen = Screen::EpisodeSelect;
                        self.show_toast(format!("Found {} episodes", self.episodes.len()), 2);
                    } else {
                        self.show_toast("No episodes found".to_string(), 3);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to load episodes: {}", e);
                    self.show_toast(format!("Error: {}", e), 5);
                }
            }
        } else {
            tracing::error!("Provider {} not found", anime.provider);
            self.show_toast("Provider not available".to_string(), 3);
        }

        Ok(())
    }

    async fn play_current_episode(&mut self) {
        self.player_controller.show_controls();

        let episode = if let Some(ep) = self.player_controller.current_episode() {
            ep.clone()
        } else {
            return;
        };

        let anime = if let Some(a) = self.player_controller.current_anime() {
            a.clone()
        } else if let Some(sa) = self.selected_anime.as_ref() {
            sa.base.clone()
        } else {
            return;
        };

        let provider_name = anime.provider.clone();
        let episode_id = format!("{}:{}", anime.id, episode.number);

        tracing::info!(
            "Playing episode {} for anime {} (provider: {})",
            episode.number,
            anime.title,
            provider_name
        );
        tracing::debug!("Episode ID format: {}", episode_id);

        self.show_toast(
            format!("Loading: {} Ep {}...", anime.title, episode.number),
            5,
        );

        // Save watch history in background (non-blocking)
        // This updates Continue Watching without delaying video playback
        let db = self.db.clone();
        let anime_clone = anime.clone();
        let episode_clone = episode.clone();
        tokio::spawn(async move {
            use chrono::Utc;
            let anime_id = format!("{}:{}", anime_clone.provider, anime_clone.id);
            let history = crate::db::WatchHistory {
                anime_id,
                provider: anime_clone.provider.clone(),
                title: anime_clone.title.clone(),
                cover_url: anime_clone.cover_url.clone(),
                episode_number: episode_clone.number,
                episode_title: episode_clone.title.clone(),
                position_seconds: 0,
                total_seconds: 0,
                updated_at: Utc::now(),
            };
            let _ = db.save_watch_history(&history).await;
            tracing::info!(
                "Saved watch history for {} ep {}",
                anime_clone.title,
                episode_clone.number
            );
        });

        // Spawn playback in background
        tokio::spawn(async move {
            let provider: Box<dyn AnimeProvider> = match provider_name.as_str() {
                "AllAnime" => Box::new(crate::providers::allanime::AllAnimeProvider::new()),
                "KKPhim" => Box::new(crate::providers::kkphim::KkphimProvider::new()),
                _ => return,
            };

            match provider.get_stream_url(&episode_id).await {
                Ok(stream_info) => {
                    if !stream_info.video_url.is_empty() {
                        let player = Player::new();
                        let _ = player.start_detached(
                            &stream_info.video_url,
                            &stream_info.subtitles,
                            &stream_info.headers,
                            None,
                        );
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to get stream URL: {}", e);
                }
            }
        });
    }

    fn show_toast(&mut self, message: String, duration_secs: u64) {
        self.toast = Some(Toast::new(message, duration_secs));
    }

    async fn on_tick(&mut self) -> Result<()> {
        if self.current_screen == Screen::Splash {
            self.splash_screen.tick();
            let elapsed = self.splash_start.elapsed().as_millis() as u64;
            let splash_duration = 2000;
            let max_splash_time = 5000;

            // Fast progress: reach 90% at 1200ms, then hold until 2000ms
            let progress = if elapsed < 1200 {
                ((elapsed * 90) / 1200).min(90) as u8
            } else if elapsed < 1800 {
                90
            } else {
                100
            };

            if elapsed < 300 {
                self.splash_screen.set_progress(progress, "Initializing...");
            } else if elapsed < 600 {
                self.splash_screen
                    .set_progress(progress, "Loading images...");
                if self.current_image_data.is_none() && !self.continue_watching.is_empty() {
                    if let Some(first) = self.continue_watching.first() {
                        if !first.cover_url.is_empty() {
                            let image_id = format!("continue_watching_{}", first.anime_id);
                            if let Ok(data) = self
                                .image_pipeline
                                .request_download(image_id, first.cover_url.clone())
                                .await
                            {
                                self.current_image_data = Some(data);
                            }
                        }
                    }
                }
            } else if elapsed < 900 {
                self.splash_screen
                    .set_progress(progress, "Caching images...");
                for history in self.continue_watching.iter().skip(1).take(3) {
                    if !history.cover_url.is_empty() {
                        let image_id = format!("continue_watching_{}", history.anime_id);
                        let _ = self
                            .image_pipeline
                            .request_download(image_id, history.cover_url.clone())
                            .await;
                    }
                }
            } else if elapsed < 1200 {
                self.splash_screen
                    .set_progress(progress, "Loading metadata...");
                if self.continue_watching_metadata.is_empty() {
                    self.load_metadata_in_background().await;
                }
            } else if elapsed < 1800 {
                self.splash_screen.set_progress(90, "Finalizing...");
            } else {
                self.splash_screen.set_progress(100, "Ready!");
            }

            // Transition to Home after exactly 2 seconds (or max timeout)
            if elapsed >= splash_duration || elapsed >= max_splash_time {
                self.current_screen = Screen::Home;
            }

            return Ok(());
        }

        // Tick loading spinner
        if !self.enriched_results.is_empty() || !self.search_overlay.query.is_empty() {
            self.loading_spinner.tick();
        }

        // Check toast expiration
        if let Some(ref toast) = self.toast {
            if toast.is_expired() {
                self.toast = None;
            }
        }

        // Check player controls timeout
        if self.player_controller.state() == PlayerState::ControlsVisible {
            if self.player_controller.controls_timeout_reached(5) {
                self.player_controller.hide_controls();
            }
        }

        // Check mpv status
        if self.current_screen == Screen::Player {
            self.player_controller.check_mpv_status();
        }

        // Refresh continue watching list when returning to Home
        if self.needs_continue_watching_refresh && self.current_screen == Screen::Home {
            self.needs_continue_watching_refresh = false;
            // Reload continue watching from database
            if let Ok(updated_list) = self.db.get_continue_watching(10).await {
                self.continue_watching = updated_list;
                // Reset selection if out of bounds
                if self.continue_watching_selected >= self.continue_watching.len()
                    && !self.continue_watching.is_empty()
                {
                    self.continue_watching_selected = 0;
                }
                // Clear current image to force reload
                self.current_image_data = None;
                self.current_anime_id = None;
            }
        }

        // Load initial image for Continue Watching when entering Home screen
        if self.current_screen == Screen::Home
            && !self.continue_watching.is_empty()
            && self.current_image_data.is_none()
        {
            self.load_continue_watching_image().await;
        }

        // Load preview image when entering search mode
        if self.needs_preview_load && self.current_screen == Screen::Search {
            self.needs_preview_load = false;
            self.load_preview().await;
        }

        // Smart auto-search with debounce (0.5 seconds)
        if self.search_pending && self.current_screen == Screen::Search {
            // Check if 500ms has passed since last keypress
            if self.last_keypress.elapsed().as_millis() >= 500 {
                self.search_pending = false;
                if self.search_overlay.query.len() >= 2 {
                    // Update UI to show searching
                    self.search_overlay.is_searching = true;

                    // Perform search
                    self.perform_search().await;

                    // Stop showing searching indicator
                    self.search_overlay.is_searching = false;
                }
            }
        }

        Ok(())
    }

    // Getter methods for fields needed by other modules
    pub(crate) fn image_renderer(&self) -> &ImageRenderer {
        &self.image_renderer
    }

    pub(crate) fn image_renderer_mut(&mut self) -> &mut ImageRenderer {
        &mut self.image_renderer
    }

    pub(crate) fn last_preview_anime_id(&self) -> &Option<String> {
        &self.last_preview_anime_id
    }

    pub(crate) fn set_last_preview_anime_id(&mut self, id: Option<String>) {
        self.last_preview_anime_id = id;
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
