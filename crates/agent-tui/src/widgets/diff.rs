use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::theme::Theme;

pub struct DiffWidget {
    diff_text: String,
    scroll: u16,
}

impl DiffWidget {
    pub fn new() -> Self {
        Self {
            diff_text: String::new(),
            scroll: 0,
        }
    }

    pub fn set_diff(&mut self, diff: String) {
        self.diff_text = diff;
        self.scroll = 0;
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let lines: Vec<Line> = self
            .diff_text
            .lines()
            .map(|line| {
                if line.starts_with('+') && !line.starts_with("+++") {
                    Line::from(Span::styled(line.to_string(), theme.diff_add()))
                } else if line.starts_with('-') && !line.starts_with("---") {
                    Line::from(Span::styled(line.to_string(), theme.diff_remove()))
                } else if line.starts_with("@@") {
                    Line::from(Span::styled(line.to_string(), theme.accent()))
                } else {
                    Line::from(line.to_string())
                }
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Diff ")
            .border_style(theme.border());

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

        frame.render_widget(paragraph, area);
    }
}

impl Default for DiffWidget {
    fn default() -> Self {
        Self::new()
    }
}
