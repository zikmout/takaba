use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Chat,
    Diff,
    Logs,
    Files,
}

impl ActivePanel {
    pub fn next(self) -> Self {
        match self {
            Self::Chat => Self::Diff,
            Self::Diff => Self::Logs,
            Self::Logs => Self::Files,
            Self::Files => Self::Chat,
        }
    }
}

pub struct AppLayout;

impl AppLayout {
    /// Compute the main layout areas.
    /// Returns (main_area, sidebar_area, status_area).
    pub fn compute(area: Rect) -> LayoutAreas {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),    // main content
                Constraint::Length(1), // status bar
            ])
            .split(area);

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70), // main panel
                Constraint::Percentage(30), // sidebar
            ])
            .split(vertical[0]);

        let sidebar = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50), // files/tools
                Constraint::Percentage(50), // logs
            ])
            .split(horizontal[1]);

        LayoutAreas {
            main: horizontal[0],
            sidebar_top: sidebar[0],
            sidebar_bottom: sidebar[1],
            status: vertical[1],
        }
    }
}

pub struct LayoutAreas {
    pub main: Rect,
    pub sidebar_top: Rect,
    pub sidebar_bottom: Rect,
    pub status: Rect,
}
