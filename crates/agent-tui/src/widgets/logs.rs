use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::theme::Theme;

pub struct LogsWidget {
    entries: Vec<LogEntry>,
    scroll: u16,
}

struct LogEntry {
    message: String,
    is_error: bool,
}

impl LogsWidget {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            scroll: 0,
        }
    }

    pub fn add_log(&mut self, message: String) {
        self.entries.push(LogEntry {
            message,
            is_error: false,
        });
    }

    pub fn add_error(&mut self, message: String) {
        self.entries.push(LogEntry {
            message,
            is_error: true,
        });
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let lines: Vec<Line> = self
            .entries
            .iter()
            .map(|e| {
                let style = if e.is_error {
                    theme.error()
                } else {
                    theme.muted()
                };
                Line::styled(e.message.clone(), style)
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Logs ")
            .border_style(theme.border());

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

        frame.render_widget(paragraph, area);
    }
}

impl Default for LogsWidget {
    fn default() -> Self {
        Self::new()
    }
}
