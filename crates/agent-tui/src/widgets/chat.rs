use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::theme::Theme;

pub struct ChatWidget {
    messages: Vec<ChatEntry>,
    streaming_buffer: String,
    scroll: u16,
}

struct ChatEntry {
    role: String,
    content: String,
}

impl ChatWidget {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            streaming_buffer: String::new(),
            scroll: 0,
        }
    }

    pub fn add_user_message(&mut self, content: String) {
        self.messages.push(ChatEntry {
            role: "user".into(),
            content,
        });
    }

    pub fn add_agent_message(&mut self, content: String) {
        self.messages.push(ChatEntry {
            role: "agent".into(),
            content,
        });
    }

    pub fn append_token(&mut self, token: &str) {
        self.streaming_buffer.push_str(token);
    }

    pub fn flush_stream(&mut self) {
        if !self.streaming_buffer.is_empty() {
            let content = std::mem::take(&mut self.streaming_buffer);
            self.messages.push(ChatEntry {
                role: "agent".into(),
                content,
            });
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut lines: Vec<Line> = Vec::new();

        for entry in &self.messages {
            let style = if entry.role == "user" {
                theme.accent()
            } else {
                theme.normal()
            };

            lines.push(Line::from(vec![Span::styled(
                format!("[{}] ", entry.role),
                style,
            )]));

            for text_line in entry.content.lines() {
                lines.push(Line::from(text_line.to_string()));
            }
            lines.push(Line::from(""));
        }

        // Streaming buffer
        if !self.streaming_buffer.is_empty() {
            lines.push(Line::from(vec![Span::styled("[agent] ", theme.accent())]));
            for text_line in self.streaming_buffer.lines() {
                lines.push(Line::from(text_line.to_string()));
            }
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Chat ")
            .border_style(theme.border());

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

        frame.render_widget(paragraph, area);
    }
}

impl Default for ChatWidget {
    fn default() -> Self {
        Self::new()
    }
}
