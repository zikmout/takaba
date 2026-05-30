use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub error: Color,
    pub success: Color,
    pub muted: Color,
    pub border: Color,
    pub highlight: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::White,
            accent: Color::Cyan,
            error: Color::Red,
            success: Color::Green,
            muted: Color::DarkGray,
            border: Color::Gray,
            highlight: Color::Yellow,
        }
    }
}

impl Theme {
    pub fn normal(&self) -> Style {
        Style::default().fg(self.fg)
    }

    pub fn accent(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn error(&self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn success(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn muted(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn border(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn highlight(&self) -> Style {
        Style::default()
            .fg(self.highlight)
            .add_modifier(Modifier::BOLD)
    }

    pub fn status_bar(&self) -> Style {
        Style::default().fg(self.bg).bg(self.accent)
    }

    pub fn diff_add(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn diff_remove(&self) -> Style {
        Style::default().fg(self.error)
    }
}
