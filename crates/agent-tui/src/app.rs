use agent_core::events::AgentEvent;
use crossterm::event::{KeyCode, KeyModifiers};

use crate::events::TuiEvent;
use crate::layout::{ActivePanel, AppLayout};
use crate::theme::Theme;
use crate::widgets::chat::ChatWidget;
use crate::widgets::diff::DiffWidget;
use crate::widgets::files::FilesWidget;
use crate::widgets::logs::LogsWidget;
use crate::widgets::status::StatusWidget;

pub struct TuiApp {
    pub should_quit: bool,
    pub active_panel: ActivePanel,
    pub chat: ChatWidget,
    pub diff: DiffWidget,
    pub files: FilesWidget,
    pub logs: LogsWidget,
    pub status: StatusWidget,
    pub theme: Theme,
    pub input: String,
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            active_panel: ActivePanel::Chat,
            chat: ChatWidget::new(),
            diff: DiffWidget::new(),
            files: FilesWidget::new(),
            logs: LogsWidget::new(),
            status: StatusWidget::new(),
            theme: Theme::default(),
            input: String::new(),
        }
    }

    pub fn handle_event(&mut self, event: TuiEvent) {
        match event {
            TuiEvent::Key(key) => self.handle_key(key),
            TuiEvent::Agent(agent_event) => self.handle_agent_event(agent_event),
            TuiEvent::Tick => {}
            TuiEvent::Resize(_, _) => {}
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        // Global shortcuts
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => self.should_quit = true,
                KeyCode::Char('d') => self.active_panel = ActivePanel::Diff,
                KeyCode::Char('t') => self.active_panel = ActivePanel::Files,
                KeyCode::Char('l') => self.active_panel = ActivePanel::Logs,
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Tab => {
                self.active_panel = self.active_panel.next();
            }
            KeyCode::Esc => {
                self.active_panel = ActivePanel::Chat;
            }
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    let msg = std::mem::take(&mut self.input);
                    self.chat.add_user_message(msg);
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            _ => {}
        }
    }

    fn handle_agent_event(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::SessionStarted(id) => {
                self.status.set_session_id(id.0);
                self.logs.add_log("session started".into());
            }
            AgentEvent::IterationStarted(i) => {
                self.status.set_iteration(i);
            }
            AgentEvent::ModelToken(token) => {
                self.chat.append_token(&token);
            }
            AgentEvent::ModelResponseComplete(_) => {
                self.chat.flush_stream();
            }
            AgentEvent::ToolCallStarted(_, name) => {
                self.status.set_active_tool(Some(name.clone()));
                self.logs.add_log(format!("tool: {name}"));
            }
            AgentEvent::ToolCallFinished(_, name) => {
                self.status.set_active_tool(None);
                self.logs.add_log(format!("done: {name}"));
            }
            AgentEvent::PatchApplied(msg) => {
                self.diff.set_diff(msg);
            }
            AgentEvent::Error(msg) => {
                self.logs.add_error(msg);
            }
            AgentEvent::SessionCompleted => {
                self.status.set_mode("completed".into());
                self.logs.add_log("session completed".into());
            }
            _ => {}
        }
    }

    pub fn render(&self, frame: &mut ratatui::Frame) {
        let areas = AppLayout::compute(frame.area());

        // Main panel - show active panel content
        match self.active_panel {
            ActivePanel::Chat => self.chat.render(frame, areas.main, &self.theme),
            ActivePanel::Diff => self.diff.render(frame, areas.main, &self.theme),
            ActivePanel::Logs => self.logs.render(frame, areas.main, &self.theme),
            ActivePanel::Files => self.files.render(frame, areas.main, &self.theme),
        }

        // Sidebar
        self.files.render(frame, areas.sidebar_top, &self.theme);
        self.logs.render(frame, areas.sidebar_bottom, &self.theme);

        // Status bar
        self.status.render(frame, areas.status, &self.theme);
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}
