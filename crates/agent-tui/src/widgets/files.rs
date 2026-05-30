use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::theme::Theme;

pub struct FilesWidget {
    files: Vec<String>,
}

impl FilesWidget {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn set_files(&mut self, files: Vec<String>) {
        self.files = files;
    }

    pub fn add_file(&mut self, file: String) {
        if !self.files.contains(&file) {
            self.files.push(file);
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self
            .files
            .iter()
            .map(|f| ListItem::new(Line::from(f.as_str())))
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Files ")
            .border_style(theme.border());

        let list = List::new(items).block(block).style(theme.normal());

        frame.render_widget(list, area);
    }
}

impl Default for FilesWidget {
    fn default() -> Self {
        Self::new()
    }
}
