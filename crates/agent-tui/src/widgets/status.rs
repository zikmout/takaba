use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

pub struct StatusWidget {
    mode: String,
    iteration: usize,
    session_id: Option<String>,
    active_tool: Option<String>,
}

impl StatusWidget {
    pub fn new() -> Self {
        Self {
            mode: "idle".into(),
            iteration: 0,
            session_id: None,
            active_tool: None,
        }
    }

    pub fn set_mode(&mut self, mode: String) {
        self.mode = mode;
    }

    pub fn set_iteration(&mut self, iteration: usize) {
        self.iteration = iteration;
    }

    pub fn set_session_id(&mut self, id: String) {
        self.session_id = Some(id);
    }

    pub fn set_active_tool(&mut self, tool: Option<String>) {
        self.active_tool = tool;
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut spans = vec![
            Span::styled(" mode: ", theme.status_bar()),
            Span::styled(&self.mode, theme.status_bar()),
            Span::styled(" | iter: ", theme.status_bar()),
            Span::styled(self.iteration.to_string(), theme.status_bar()),
        ];

        if let Some(ref tool) = self.active_tool {
            spans.push(Span::styled(" | tool: ", theme.status_bar()));
            spans.push(Span::styled(tool.clone(), theme.status_bar()));
        }

        if let Some(ref id) = self.session_id {
            spans.push(Span::styled(" | session: ", theme.status_bar()));
            spans.push(Span::styled(&id[..8.min(id.len())], theme.status_bar()));
        }

        let line = Line::from(spans);

        let block = Block::default().borders(Borders::NONE);

        let paragraph = Paragraph::new(vec![line])
            .block(block)
            .style(theme.status_bar());

        frame.render_widget(paragraph, area);
    }
}

impl Default for StatusWidget {
    fn default() -> Self {
        Self::new()
    }
}
