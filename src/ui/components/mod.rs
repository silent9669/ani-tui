use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::time::Instant;

pub mod episode_grid;
pub use episode_grid::EpisodeGrid;

pub struct AnimationState {
    pub progress: f32,
    pub started_at: Instant,
    pub duration_ms: u64,
    pub easing: EasingFunction,
}

#[derive(Clone, Copy)]
pub enum EasingFunction {
    Linear,
    EaseInOut,
    EaseOut,
}

impl AnimationState {
    pub fn new(duration_ms: u64, easing: EasingFunction) -> Self {
        Self {
            progress: 0.0,
            started_at: Instant::now(),
            duration_ms,
            easing,
        }
    }

    pub fn tick(&mut self) -> f32 {
        let elapsed = self.started_at.elapsed().as_millis() as f32;
        let raw = (elapsed / self.duration_ms as f32).min(1.0);
        self.progress = match self.easing {
            EasingFunction::EaseInOut => {
                if raw < 0.5 {
                    2.0 * raw * raw
                } else {
                    1.0 - (-2.0 * raw + 2.0).powi(2) / 2.0
                }
            }
            EasingFunction::EaseOut => 1.0 - (1.0 - raw).powi(2),
            EasingFunction::Linear => raw,
        };
        self.progress
    }

    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }

    pub fn reset(&mut self) {
        self.started_at = Instant::now();
        self.progress = 0.0;
    }
}

pub struct Theme;

impl Theme {
    pub fn primary() -> Color {
        Color::Rgb(229, 9, 20)
    }

    pub fn secondary() -> Color {
        Color::Rgb(108, 52, 131)
    }

    pub fn accent() -> Color {
        Color::Rgb(0, 212, 255)
    }

    pub fn surface() -> Color {
        Color::Rgb(26, 26, 46)
    }

    pub fn text() -> Color {
        Color::Rgb(224, 224, 224)
    }

    pub fn text_dim() -> Color {
        Color::Rgb(128, 128, 128)
    }

    pub fn success() -> Color {
        Color::Rgb(46, 204, 113)
    }

    pub fn warning() -> Color {
        Color::Rgb(243, 156, 18)
    }

    pub fn gradient_start() -> Color {
        Color::Rgb(255, 0, 128)
    }

    pub fn gradient_end() -> Color {
        Color::Rgb(0, 128, 255)
    }
}

pub struct LoadingSpinner {
    frames: Vec<String>,
    current_frame: usize,
}

impl Default for LoadingSpinner {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadingSpinner {
    pub fn new() -> Self {
        Self {
            frames: vec![
                "◐".to_string(),
                "◓".to_string(),
                "◑".to_string(),
                "◒".to_string(),
            ],
            current_frame: 0,
        }
    }

    pub fn tick(&mut self) {
        self.current_frame = (self.current_frame + 1) % self.frames.len();
    }

    pub fn render(&self) -> Line<'_> {
        Line::from(vec![
            Span::styled(
                &self.frames[self.current_frame],
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(" "),
        ])
    }
}

pub struct ProgressBar;

impl ProgressBar {
    pub fn render(progress_pct: u8, width: usize) -> String {
        let filled = (progress_pct as usize * width) / 100;
        let empty = width - filled;

        format!(
            "[{}{}] {}%",
            "█".repeat(filled),
            "░".repeat(empty),
            progress_pct
        )
    }
}

pub struct Toast {
    pub message: String,
    pub duration_secs: u64,
    pub created_at: std::time::Instant,
}

impl Toast {
    pub fn new(message: String, duration_secs: u64) -> Self {
        Self {
            message,
            duration_secs,
            created_at: std::time::Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() >= self.duration_secs
    }
}
