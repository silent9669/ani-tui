use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

pub struct LoadingSpinner {
    frames: Vec<String>,
    current_frame: usize,
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

    pub fn tick(&mut self,
    ) {
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

    pub fn is_expired(&self,
    ) -> bool {
        self.created_at.elapsed().as_secs() >= self.duration_secs
    }
}
