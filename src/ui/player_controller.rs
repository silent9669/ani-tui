use std::process::Child;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerState {
    Playing,
    ControlsVisible,
    Ended,
}

pub struct PlayerController {
    state: PlayerState,
    current_anime: Option<(crate::providers::Anime, Vec<crate::providers::Episode>)>,
    current_episode_idx: usize,
    mpv_process: Option<Child>,
    controls_since: Option<Instant>,
    selected_control: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlAction {
    NextEpisode,
    PreviousEpisode,
    ChooseEpisode,
    BackToMenu,
}

const CONTROLS: &[(ControlAction, &str, char)] = &[
    (ControlAction::NextEpisode, "Next Episode", 'n'),
    (ControlAction::PreviousEpisode, "Previous Episode", 'p'),
    (ControlAction::ChooseEpisode, "Choose Episode", 'e'),
    (ControlAction::BackToMenu, "Back to Menu", 'b'),
];

impl PlayerController {
    pub fn new() -> Self {
        Self {
            state: PlayerState::Playing,
            current_anime: None,
            current_episode_idx: 0,
            mpv_process: None,
            controls_since: None,
            selected_control: 0,
        }
    }

    pub fn state(&self) -> PlayerState {
        self.state
    }

    pub fn start_playback(
        &mut self,
        anime: crate::providers::Anime,
        episodes: Vec<crate::providers::Episode>,
        start_episode: usize,
    ) {
        self.current_anime = Some((anime, episodes));
        self.current_episode_idx = start_episode;
        self.state = PlayerState::Playing;
        self.controls_since = None;
        self.selected_control = 0;
    }

    pub fn show_controls(&mut self) {
        self.state = PlayerState::ControlsVisible;
        self.controls_since = Some(Instant::now());
    }

    pub fn hide_controls(&mut self) {
        self.state = PlayerState::Playing;
        self.controls_since = None;
    }

    pub fn on_video_end(&mut self) {
        self.state = PlayerState::Ended;
    }

    pub fn next_control(&mut self) {
        self.selected_control = (self.selected_control + 1) % CONTROLS.len();
    }

    pub fn previous_control(&mut self) {
        if self.selected_control == 0 {
            self.selected_control = CONTROLS.len() - 1;
        } else {
            self.selected_control -= 1;
        }
    }

    pub fn get_selected_action(&self) -> ControlAction {
        CONTROLS[self.selected_control].0
    }

    pub fn current_episode(&self) -> Option<&crate::providers::Episode> {
        self.current_anime
            .as_ref()
            .and_then(|(_, eps)| eps.get(self.current_episode_idx))
    }

    pub fn has_next_episode(&self) -> bool {
        self.current_anime
            .as_ref()
            .map(|(_, eps)| self.current_episode_idx + 1 < eps.len())
            .unwrap_or(false)
    }

    pub fn has_previous_episode(&self) -> bool {
        self.current_episode_idx > 0
    }

    pub fn play_next_episode(&mut self) -> bool {
        if self.has_next_episode() {
            self.current_episode_idx += 1;
            self.state = PlayerState::Playing;
            self.controls_since = None;
            true
        } else {
            false
        }
    }

    pub fn play_previous_episode(&mut self) -> bool {
        if self.has_previous_episode() {
            self.current_episode_idx -= 1;
            self.state = PlayerState::Playing;
            self.controls_since = None;
            true
        } else {
            false
        }
    }

    pub fn select_episode(&mut self, idx: usize) {
        if let Some((_, eps)) = &self.current_anime {
            if idx < eps.len() {
                self.current_episode_idx = idx;
                self.state = PlayerState::Playing;
                self.controls_since = None;
            }
        }
    }

    pub fn set_mpv_process(&mut self, process: Child) {
        self.mpv_process = Some(process);
    }

    pub fn check_mpv_status(&mut self) -> Option<std::process::ExitStatus> {
        if let Some(ref mut process) = self.mpv_process {
            match process.try_wait() {
                Ok(Some(status)) => {
                    self.mpv_process = None;
                    if self.state == PlayerState::Playing {
                        self.on_video_end();
                    }
                    Some(status)
                }
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn anime_title(&self) -> Option<&str> {
        self.current_anime.as_ref().map(|(a, _)| a.title.as_str())
    }

    pub fn episode_number(&self) -> u32 {
        self.current_anime
            .as_ref()
            .and_then(|(_, eps)| eps.get(self.current_episode_idx))
            .map(|ep| ep.number)
            .unwrap_or(1)
    }

    pub fn current_anime(&self) -> Option<&crate::providers::Anime> {
        self.current_anime.as_ref().map(|(a, _)| a)
    }

    pub fn total_episodes(&self) -> usize {
        self.current_anime
            .as_ref()
            .map(|(_, eps)| eps.len())
            .unwrap_or(0)
    }

    pub fn controls_timeout_reached(&self, timeout_secs: u64) -> bool {
        self.controls_since
            .map(|since| since.elapsed().as_secs() >= timeout_secs)
            .unwrap_or(false)
    }

    pub fn current_episode_idx(&self) -> usize {
        self.current_episode_idx
    }

    pub fn selected_control(&self) -> usize {
        self.selected_control
    }
}

pub struct EndScreen;

impl EndScreen {
    pub fn render(has_next: bool) -> Vec<(ratatui::text::Line<'static>, Option<EndScreenAction>)> {
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};

        let mut options = vec![];

        if has_next {
            options.push((
                Line::from(vec![
                    Span::raw("▶ "),
                    Span::styled(
                        "Play Next Episode",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Some(EndScreenAction::NextEpisode),
            ));
        }

        options.push((
            Line::from(vec![
                Span::raw("↺ "),
                Span::styled("Replay This Episode", Style::default().fg(Color::Yellow)),
            ]),
            Some(EndScreenAction::Replay),
        ));

        options.push((
            Line::from(vec![
                Span::raw("← "),
                Span::styled("Back to Menu", Style::default().fg(Color::Gray)),
            ]),
            Some(EndScreenAction::BackToMenu),
        ));

        options
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EndScreenAction {
    NextEpisode,
    Replay,
    BackToMenu,
}

pub struct EpisodeListModal;

impl EpisodeListModal {
    pub fn render(
        episodes: &[crate::providers::Episode],
        current_idx: usize,
        scroll_offset: usize,
        visible_count: usize,
    ) -> Vec<(ratatui::text::Line<'static>, usize)> {
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};

        let mut lines = vec![];
        let end_idx = (scroll_offset + visible_count).min(episodes.len());

        for (idx, ep) in episodes
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(end_idx - scroll_offset)
        {
            let is_current = idx == current_idx;
            let is_selected = idx == current_idx;

            let prefix = if is_selected { "▶ " } else { "  " };

            let style = if is_current {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let ep_title = ep
                .title
                .as_ref()
                .map(|t| format!("{}", t))
                .unwrap_or_else(|| format!("Episode {}", ep.number));

            lines.push((
                Line::from(vec![Span::raw(prefix), Span::styled(ep_title, style)]),
                idx,
            ));
        }

        lines
    }
}

impl Default for PlayerController {
    fn default() -> Self {
        Self::new()
    }
}
