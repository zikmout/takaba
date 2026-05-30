#![allow(dead_code)]

mod app;
mod events;
mod layout;
mod theme;
mod widgets;

use std::io;

use agent_core::events::EventBus;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;

use crate::app::TuiApp;
use crate::events::TuiEventHandler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Create event bus and TUI app
    let event_bus = EventBus::default();
    let agent_rx = event_bus.subscribe();
    let mut event_handler = TuiEventHandler::new(agent_rx);
    let mut app = TuiApp::new();

    // Main loop
    loop {
        terminal.draw(|frame| {
            app.render(frame);
        })?;

        if let Some(event) = event_handler.next().await {
            app.handle_event(event);
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
