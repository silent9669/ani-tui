use crate::config::Config;
use crate::db::Database;
use crate::image::{AsciiRenderer, ImagePipeline};
use crate::metadata::{EnrichedAnime, MetadataCache};
use crate::player::Player;
use crate::providers::{AnimeProvider, Episode, Language, ProviderRegistry};
use crate::ui::components::{LoadingSpinner, Toast};
use crate::ui::modern_components::{PreviewPanel, SearchOverlay, SourceSelectModal, SplashScreen};
use crate::ui::player_controller::{ControlAction, PlayerController, PlayerState};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
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
    #[allow(dead_code)]
    chafa_renderer: AsciiRenderer,

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

    // UI Components
    loading_spinner: LoadingSpinner,
    toast: Option<Toast>,

    // Image cache for current preview
    current_image_data: Option<Vec<u8>>,

    // Track previous screen for navigation
    previous_screen: Option<Screen>,

    // Trigger preview load when entering search
    needs_preview_load: bool,

    // Episode list modal in player
    show_episode_list: bool,
    episode_list_scroll: usize,

    // Search debounce
    search_pending: bool,
    #[allow(dead_code)]
    last_keypress: Instant,
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
        let images_to_preload: Vec<(String, String)> = continue_watching.iter()
            .filter(|h| !h.cover_url.is_empty())
            .map(|h| (format!("continue_watching_{}", h.anime_id), h.cover_url.clone()))
            .collect();
        let _ = image_pipeline.preload_images(images_to_preload);

        // Load first image immediately if available
        let mut current_image_data = None;
        if let Some(first) = continue_watching.first() {
            if !first.cover_url.is_empty() {
                let image_id = format!("continue_watching_{}", first.anime_id);
                match image_pipeline.request_download(image_id, first.cover_url.clone()).await {
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

        // AsciiRenderer is always available (pure Rust crate)
        // No external dependencies required

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
            chafa_renderer: AsciiRenderer::new(),
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
            current_image_data,
            previous_screen: None,
            needs_preview_load: false,
            show_episode_list: false,
            episode_list_scroll: 0,
            search_pending: false,
            last_keypress: Instant::now(),
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
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.run_app(&mut terminal).await;

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
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
                        // Handle Shift+S for search
                        if key.code == KeyCode::Char('S') {
                            if self.current_screen == Screen::Home {
                                self.current_screen = Screen::Search;
                                self.needs_preview_load = true;
                                continue;
                            }
                        }
                        
                        // Handle Shift+C for source toggle during search
                        if key.code == KeyCode::Char('C') {
                            if self.current_screen == Screen::Search {
                                self.show_source_modal = !self.show_source_modal;
                                continue;
                            }
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

        let sources: Vec<(String, Language, bool)> = vec![
            ("AllAnime".to_string(), Language::English, self.selected_source == Language::English),
            ("KKPhim".to_string(), Language::Vietnamese, self.selected_source == Language::Vietnamese),
        ];

        SourceSelectModal::render(frame, area, &sources, self.selected_source_idx);
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
        let status_bar = Paragraph::new("↑/↓: Navigate | Enter: Resume | Shift+D: Remove | Shift+S: Search | ESC: Quit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(status_bar, main_chunks[1]);
    }

    fn draw_continue_watching(&self,
        frame: &mut Frame,
        area: Rect,
    ) {
        let mut history_lines: Vec<Line> = Vec::new();

        for (idx, history) in self.continue_watching.iter().take(5).enumerate() {
            let is_selected = idx == self.continue_watching_selected;
            
            let prefix = if is_selected { "▶ " } else { "  " };
            let title_style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };

            history_lines.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled(&history.title, title_style),
                Span::raw(format!(" - Episode {}", history.episode_number)),
                Span::styled("  *last watched", Style::default().fg(Color::Red)),
            ]));
        }

        let history_widget = Paragraph::new(Text::from(history_lines))
            .block(Block::default().borders(Borders::ALL).title("Continue Watching"));

        frame.render_widget(history_widget, area);
    }

    fn draw_continue_watching_preview(&self,
        frame: &mut Frame,
        area: Rect,
    ) {
        if let Some(history) = self.continue_watching.get(self.continue_watching_selected) {
            // Split into image and info - portrait orientation
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(60), // Image area (portrait orientation)
                    Constraint::Percentage(40), // Info area
                ])
                .margin(1)
                .split(area);

            // Show cover image using chafa if available
            let image_block = Block::default()
                .borders(Borders::ALL)
                .title("Cover Image");
            
            if let Some(image_data) = &self.current_image_data {
                // Try to render with chafa
                Self::render_image_with_ascii(frame, chunks[0], image_data);
            } else {
                // Show placeholder
                let image_text = if !history.cover_url.is_empty() {
                    "[Loading image...]"
                } else {
                    "[No Image Available]"
                };
                
                let image_widget = Paragraph::new(image_text)
                    .alignment(Alignment::Center)
                    .block(image_block);
                frame.render_widget(image_widget, chunks[0]);
            }

            // Show info
            let mut info_lines: Vec<Line> = Vec::new();
            
            // Title
            info_lines.push(Line::from(vec![Span::styled(
                &history.title,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));
            
            info_lines.push(Line::from(""));
            
            // Episode info
            info_lines.push(Line::from(vec![
                Span::raw("Episode: "),
                Span::styled(history.episode_number.to_string(), Style::default().fg(Color::Green)),
            ]));
            
            info_lines.push(Line::from(vec![
                Span::raw("Provider: "),
                Span::styled(&history.provider, Style::default().fg(Color::Yellow)),
            ]));
            
            info_lines.push(Line::from(""));
            info_lines.push(Line::from("Press Enter to resume watching"));

            let info_widget = Paragraph::new(Text::from(info_lines))
                .block(Block::default().borders(Borders::ALL).title("Preview"));
            frame.render_widget(info_widget, chunks[1]);
        }
    }

    fn render_image_with_ascii(frame: &mut Frame, area: Rect, image_data: &[u8]) {
        use crate::image::AsciiRenderer;
        use crate::ui::supports_images;

        let supports_image = supports_images();

        if supports_image && !image_data.is_empty() {
            // For terminals that support images, show a clean placeholder
            let image_block = Block::default()
                .borders(Borders::ALL)
                .title("Cover Image");

            let placeholder = Paragraph::new("Press Enter to view image")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray))
                .block(image_block);

            frame.render_widget(placeholder, area);
        } else {
            // Fallback to ASCII rendering for terminals without image support
            let renderer = AsciiRenderer::new();
            let width = area.width.saturating_sub(2) as u32;
            let height = area.height.saturating_sub(2) as u32;

            match renderer.render(image_data, width, height) {
                Ok(rendered) => {
                    let lines: Vec<Line> = rendered
                        .lines()
                        .take(area.height as usize)
                        .map(|line| Line::from(line.to_string()))
                        .collect();

                    let image_widget = Paragraph::new(Text::from(lines));
                    frame.render_widget(image_widget, area);
                }
                Err(e) => {
                    let placeholder = Paragraph::new(format!("[Image error: {}]", e))
                        .alignment(Alignment::Center);
                    frame.render_widget(placeholder, area);
                }
            }
        }
    }

    fn draw_search(&mut self, frame: &mut Frame) {
        let area = frame.size();

        // Split into search results and preview
        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Draw search overlay in left panel
        self.search_overlay.results = self.enriched_results.clone();
        self.search_overlay.selected_index = self.selected_index;
        self.search_overlay.is_searching = self.search_overlay.query.len() >= 2 && self.enriched_results.is_empty();
        self.search_overlay.render(frame, main_layout[0], &[self.selected_source]);

        // Draw preview panel in right panel
        let selected_anime = self.enriched_results.get(self.selected_index);
        let image_data = self.current_image_data.as_ref().map(|d| std::slice::from_ref(d));
        PreviewPanel::render(frame, main_layout[1], selected_anime, image_data);
    }

    fn draw_episode_select(&mut self, frame: &mut Frame) {
        let area = frame.size();

        // Get anime title and check for last watched episode
        let (title, last_watched_ep) = self.selected_anime.as_ref()
            .map(|a| {
                let anime_id = format!("{}:{}", a.base.provider, a.base.id);
                let last_ep = self.continue_watching.iter()
                    .find(|h| h.anime_id == anime_id)
                    .map(|h| h.episode_number);
                (a.base.title.clone(), last_ep)
            })
            .unwrap_or_else(|| ("Select Episode".to_string(), None));

        // Split area into info (top) and episodes (bottom)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(0),     // Episodes list
            ])
            .split(area);

        // Header with anime title
        let header = Paragraph::new(format!("{} - Select Episode to Watch", title))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(header, chunks[0]);

        // Episodes list
        let mut lines: Vec<Line> = Vec::new();
        
        for (idx, episode) in self.episodes.iter().enumerate() {
            let is_selected = idx == self.episode_list_scroll;
            let is_last_watched = last_watched_ep.map(|ep| ep == episode.number).unwrap_or(false);
            
            let prefix = if is_selected { "▶ " } else { "  " };
            let ep_title = episode.title.as_ref()
                .map(|t| t.clone())
                .unwrap_or_else(|| format!("Episode {}", episode.number));
            
            // Use red color for last watched episodes
            let style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if is_last_watched {
                Style::default().fg(Color::Red)
            } else {
                Style::default()
            };

            // Display episode title or "Episode X" if no title
            let display_text = if ep_title.starts_with("Episode ") {
                ep_title.to_string()
            } else {
                format!("Episode {}: {}", episode.number, ep_title)
            };
            
            let mut spans = vec![
                Span::raw(prefix),
                Span::styled(display_text, style),
            ];
            
            // Add "*last watched" indicator if this is the last watched episode
            if is_last_watched {
                spans.push(Span::styled("  *last watched", Style::default().fg(Color::Red)));
            }
            
            lines.push(Line::from(spans));
        }

        if self.episodes.is_empty() {
            lines.push(Line::from("No episodes available"));
        }

        lines.push(Line::from(""));
        lines.push(Line::from("↑/↓: Navigate | Enter: Play | Esc: Back"));

        let episodes_widget = Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::ALL).title(format!("Episodes ({})", self.episodes.len())));
        frame.render_widget(episodes_widget, chunks[1]);
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

        // Show control overlay if visible
        if self.player_controller.state() == PlayerState::ControlsVisible {
            self.draw_control_overlay(frame, area);
        } else {
            // Show minimal indicator that player is active
            let paragraph = Paragraph::new("Video playing... Press any key for controls")
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Player"));
            frame.render_widget(paragraph, area);
        }

        // Episode list modal
        if self.show_episode_list {
            self.draw_episode_list_modal(frame);
        }
    }

    fn draw_control_overlay(&self, frame: &mut Frame, area: Rect) {
        let controls = vec![
            ("Next Episode", "n", self.player_controller.has_next_episode()),
            ("Previous Episode", "p", self.player_controller.has_previous_episode()),
            ("Choose Episode", "e", true),
            ("Download", "d", false), // TODO: implement download
            ("Back to Menu", "b", true),
        ];

        let mut lines: Vec<Line> = vec![
            Line::from(vec![
                Span::styled(
                    format!("{} - Episode {}/{}",
                        self.player_controller.anime_title().unwrap_or("Unknown"),
                        self.player_controller.episode_number(),
                        self.player_controller.total_episodes()
                    ),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
        ];

        for (idx, (label, key, enabled)) in controls.iter().enumerate() {
            let is_selected = idx == self.player_controller.selected_control();
            
            let style = if !enabled {
                Style::default().fg(Color::DarkGray)
            } else if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let prefix = if is_selected { "▶ " } else { "  " };

            lines.push(Line::from(vec![
                Span::styled(format!("{}[{}] ", prefix, key), style),
                Span::styled(*label, style),
            ]));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Controls (auto-hide in 5s)");

        let paragraph = Paragraph::new(Text::from(lines))
            .block(block)
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    fn draw_episode_list_modal(&mut self, frame: &mut Frame) {
        let area = centered_rect(60, 70, frame.size());
        
        frame.render_widget(Clear, area);
        
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Choose Episode");

        let visible_count = (area.height as usize).saturating_sub(2);
        let episodes = &self.episodes;
        let current_ep = self.player_controller.current_episode_idx();
        
        let lines = crate::ui::player_controller::EpisodeListModal::render(
            episodes,
            current_ep,
            self.episode_list_scroll,
            visible_count,
        );

        let text: Vec<Line> = lines.into_iter().map(|(line, _)| line).collect();
        
        let paragraph = Paragraph::new(Text::from(text))
            .block(block);

        frame.render_widget(paragraph, area);
    }

    fn draw_end_screen(&mut self, frame: &mut Frame) {
        let area = frame.size();

        let options = crate::ui::player_controller::EndScreen::render(
            self.player_controller.has_next_episode()
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
        let area = centered_rect(50, 40, frame.size());
        
        frame.render_widget(Clear, area);

        let sources: Vec<(String, Language, bool)> = vec![
            ("AllAnime (English)".to_string(), Language::English, self.selected_source == Language::English),
            ("KKPhim (Vietnamese)".to_string(), Language::Vietnamese, self.selected_source == Language::Vietnamese),
        ];

        SourceSelectModal::render(frame, area, &sources, self.selected_source_idx);
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

    async fn handle_key(
        &mut self,
        key: KeyCode,
    ) -> Result<()> {
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

    async fn handle_episode_select_key(
        &mut self,
        key: KeyCode,
    ) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Char('b') => {
                // Go back to previous screen (Dashboard or Search)
                let target = self.previous_screen.take().unwrap_or(Screen::Home);
                self.current_screen = target;
                self.episodes.clear();
            }
            KeyCode::Up => {
                if self.episode_list_scroll > 0 {
                    self.episode_list_scroll -= 1;
                }
            }
            KeyCode::Down => {
                if self.episode_list_scroll < self.episodes.len().saturating_sub(1) {
                    self.episode_list_scroll += 1;
                }
            }
            KeyCode::Enter => {
                // Play selected episode
                if let Some(anime) = self.selected_anime.as_ref().map(|a| a.base.clone()) {
                    if self.episode_list_scroll < self.episodes.len() {
                        self.player_controller.start_playback(
                            anime,
                            self.episodes.clone(),
                            self.episode_list_scroll,
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

    async fn handle_splash_key(
        &mut self,
        key: KeyCode,
    ) -> Result<()> {
        if key == KeyCode::Enter || key == KeyCode::Esc {
            self.current_screen = Screen::SourceSelect;
        }
        Ok(())
    }

    async fn handle_source_select_key(
        &mut self,
        key: KeyCode,
    ) -> Result<()> {
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
                // Set source based on selection and go home
                let new_source = if self.selected_source_idx == 0 {
                    Language::English
                } else {
                    Language::Vietnamese
                };
                if new_source != self.selected_source {
                    tracing::info!("Source changed from {:?} to {:?}", self.selected_source, new_source);
                    self.selected_source = new_source;
                }
                self.current_screen = Screen::Home;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_home_key(
        &mut self,
        key: KeyCode,
    ) -> Result<()> {
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
                if let Some(history) = self.continue_watching.get(self.continue_watching_selected).cloned() {
                    self.resume_watching(history).await?;
                }
            }
            KeyCode::Char('D') => {
                // Remove from continue watching
                if !self.continue_watching.is_empty() && self.continue_watching_selected < self.continue_watching.len() {
                    let history = &self.continue_watching[self.continue_watching_selected];
                    let anime_id = history.anime_id.clone();
                    let title = history.title.clone();
                    
                    // Remove from database
                    let _ = self.db.remove_from_continue_watching(&anime_id).await;
                    
                    // Remove from local list
                    self.continue_watching.remove(self.continue_watching_selected);
                    
                    // Adjust selection
                    if self.continue_watching_selected >= self.continue_watching.len() && self.continue_watching_selected > 0 {
                        self.continue_watching_selected -= 1;
                    }
                    
                    self.show_toast(format!("Removed '{}' from Continue Watching", title), 2);
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn resume_watching(
        &mut self,
        history: crate::db::WatchHistory,
    ) -> Result<()> {
        tracing::info!("Resuming anime: {} - Ep {}", history.title, history.episode_number);
        
        // Find the provider for this history entry
        if let Some(_provider) = self.providers.get_provider(&history.provider) {
            // Create a basic Anime struct from history
            let anime = crate::providers::Anime {
                id: history.anime_id.split(':').nth(1).unwrap_or(&history.anime_id).to_string(),
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
        if let Some(history) = self.continue_watching.get(self.continue_watching_selected) {
            if !history.cover_url.is_empty() {
                let image_id = format!("continue_watching_{}", history.anime_id);
                let cover_url = history.cover_url.clone();

                // Try to get image from cache or download it
                match self.image_pipeline.request_download(image_id, cover_url).await {
                    Ok(data) => {
                        self.current_image_data = Some(data);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load cover image: {}", e);
                        self.current_image_data = None;
                    }
                }
            } else {
                self.current_image_data = None;
            }
        }
    }

    async fn handle_search_key(
        &mut self,
        key: KeyCode,
    ) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Char('B') => {
                self.current_screen = Screen::Home;
                self.search_overlay.query.clear();
                self.enriched_results.clear();
                self.search_pending = false;
                self.last_keypress = Instant::now();
            }
            KeyCode::Backspace => {
                self.search_overlay.query.pop();
                self.search_pending = true;
                self.last_keypress = Instant::now();
                // Clear results if empty
                if self.search_overlay.query.is_empty() {
                    self.enriched_results.clear();
                    self.search_pending = false;
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
                // Select the current anime and go to episode list
                if let Some(anime) = self.enriched_results.get(self.selected_index).cloned() {
                    self.selected_anime = Some(anime.clone());
                    self.select_anime(anime.base).await?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_episode_list_key(
        &mut self,
        key: KeyCode,
    ) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.show_episode_list = false;
            }
            KeyCode::Up => {
                if self.episode_list_scroll > 0 {
                    self.episode_list_scroll -= 1;
                }
            }
            KeyCode::Down => {
                let visible = 20; // Approximate visible count
                if self.episode_list_scroll + visible < self.episodes.len() {
                    self.episode_list_scroll += 1;
                }
            }
            KeyCode::Enter => {
                // Play selected episode
                let selected = self.episode_list_scroll;
                if selected < self.episodes.len() {
                    self.player_controller.select_episode(selected);
                    self.play_current_episode().await;
                }
                self.show_episode_list = false;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_player_key(
        &mut self,
        key: KeyCode,
    ) -> Result<()> {
        match self.player_controller.state() {
            PlayerState::Playing => {
                // Any key shows controls
                self.player_controller.show_controls();
            }
            PlayerState::ControlsVisible => {
                match key {
                    KeyCode::Esc => {
                        self.player_controller.hide_controls();
                    }
                    KeyCode::Up => {
                        self.player_controller.previous_control();
                    }
                    KeyCode::Down => {
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
                        self.show_episode_list = true;
                        self.episode_list_scroll = self.player_controller.current_episode_idx();
                    }
                    KeyCode::Char('b') => {
                        self.save_watch_history().await;
                        self.current_screen = Screen::Home;
                        self.player_controller = PlayerController::new();
                    }
                    _ => {}
                }
            }
            PlayerState::Ended => {
                match key {
                    KeyCode::Esc | KeyCode::Char('b') => {
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

    async fn handle_source_modal_key(
        &mut self,
        key: KeyCode,
    ) -> Result<()> {
        match key {
            KeyCode::Esc => {
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
                // Set source based on selection and close modal
                let new_source = if self.selected_source_idx == 0 {
                    Language::English
                } else {
                    Language::Vietnamese
                };
                if new_source != self.selected_source {
                    tracing::info!("Source changed from {:?} to {:?}", self.selected_source, new_source);
                    self.selected_source = new_source;
                }
                self.show_source_modal = false;
                // Re-run search with new source
                if !self.search_overlay.query.is_empty() {
                    self.perform_search().await;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn execute_control_action(&mut self,
    ) {
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
                self.show_episode_list = true;
                self.episode_list_scroll = self.player_controller.current_episode_idx();
            }
            ControlAction::Download => {
                self.show_toast("Download not yet implemented".to_string(), 3);
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
        
        tracing::info!("Searching for '{}' with source: {:?}", query, self.selected_source);
        
        // Search selected source only
        let mut all_results = Vec::new();
        
        match self.providers.search_filtered(&query, &[self.selected_source]).await {
            Ok(mut results) => {
                tracing::info!("Found {} results from {:?}", results.len(), self.selected_source);
                all_results.append(&mut results);
            }
            Err(e) => {
                tracing::warn!("Search failed for {:?}: {}", self.selected_source, e);
            }
        }
        
        // Update results
        let enriched: Vec<_> = all_results.into_iter()
            .map(|base| crate::metadata::EnrichedAnime { base, metadata: None })
            .collect();
        
        self.enriched_results = enriched;
        self.selected_index = 0;
        
        // Load preview for first result
        if !self.enriched_results.is_empty() {
            self.load_preview().await;
        }
    }

    async fn load_preview(&mut self) {
        if let Some(anime) = self.enriched_results.get_mut(self.selected_index) {
            // Load image
            let id = anime.base.id.clone();
            let url = anime.base.cover_url.clone();
            
            // Try to get image from cache or download
            if let Some(image) = self.image_pipeline.get_image(&id).await {
                self.current_image_data = Some(image.data);
            } else if !url.is_empty() {
                // Request download (don't block UI if it fails)
                let image_result = self.image_pipeline.request_download(id.clone(), url.clone()).await;
                if let Ok(data) = image_result {
                    self.current_image_data = Some(data);
                } else {
                    self.current_image_data = None;
                }
            }
            
            // Fetch metadata if not already loaded
            if anime.metadata.is_none() {
                if let Ok(metadata_list) = self.metadata_cache.search_and_cache(&anime.base.title).await {
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
                tracing::info!("Saved watch history for {} ep {}", anime.base.title, episode.number);
            }
        }
    }

    async fn select_anime(
        &mut self,
        anime: crate::providers::Anime,
    ) -> Result<()> {
        tracing::info!("Selecting anime: {} from provider: {}", anime.title, anime.provider);
        
        self.selected_anime = Some(crate::metadata::EnrichedAnime {
            base: anime.clone(),
            metadata: None,
        });
        self.episodes.clear();
        self.show_toast(format!("Loading episodes for {}...", anime.title), 3);

        // Find last watched episode position for this anime
        let anime_id = format!("{}:{}", anime.provider, anime.id);
        let last_watched_ep = self.continue_watching.iter()
            .find(|h| h.anime_id == anime_id)
            .map(|h| h.episode_number);

        // Load episodes from the provider
        if let Some(provider) = self.providers.get_provider(&anime.provider) {
            tracing::info!("Found provider, loading episodes for anime_id: {}", anime.id);
            match provider.get_episodes(&anime.id).await {
                Ok(episodes) => {
                    tracing::info!("Loaded {} episodes", episodes.len());
                    self.episodes = episodes;
                    if !self.episodes.is_empty() {
                        // Set scroll position to last watched episode, or 0 if not found
                        self.episode_list_scroll = last_watched_ep
                            .and_then(|ep| self.episodes.iter().position(|e| e.number == ep))
                            .unwrap_or(0);
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

    async fn play_current_episode(&mut self,
    ) {
        if let Some(episode) = self.player_controller.current_episode() {
            let anime = self.player_controller.current_anime()
                .cloned()
                .unwrap_or_else(|| {
                    self.selected_anime.as_ref().unwrap().base.clone()
                });

            let provider_name = anime.provider.clone();
            let episode_id = format!("{}:{}", anime.id, episode.number);
            
            tracing::info!("Playing episode {} for anime {} (provider: {})", 
                episode.number, anime.title, provider_name);
            tracing::debug!("Episode ID format: {}", episode_id);

            self.show_toast(format!("Loading: {} Ep {}...", anime.title, episode.number), 5);

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
    }

    fn show_toast(
        &mut self,
        message: String,
        duration_secs: u64,
    ) {
        self.toast = Some(Toast::new(message, duration_secs));
    }

    async fn on_tick(
        &mut self,
    ) -> Result<()> {
        // Update splash screen
        if self.current_screen == Screen::Splash {
            self.splash_screen.tick();
            if self.splash_screen.is_complete(self.splash_start.elapsed().as_millis() as u64) {
                self.current_screen = Screen::SourceSelect;
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

        // Load initial image for Continue Watching when entering Home screen
        if self.current_screen == Screen::Home && !self.continue_watching.is_empty() && self.current_image_data.is_none() {
            self.load_continue_watching_image().await;
        }

        // Load preview image when entering search mode
        if self.needs_preview_load && self.current_screen == Screen::Search {
            self.needs_preview_load = false;
            self.load_preview().await;
        }

        // Smart auto-search with debounce
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