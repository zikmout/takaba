use agent_core::events::AgentEvent;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

pub enum TuiEvent {
    Key(KeyEvent),
    Agent(AgentEvent),
    Tick,
    Resize(u16, u16),
}

pub struct TuiEventHandler {
    rx: mpsc::UnboundedReceiver<TuiEvent>,
}

impl TuiEventHandler {
    pub fn new(agent_rx: tokio::sync::broadcast::Receiver<AgentEvent>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        // Crossterm events
        let tx_key = tx.clone();
        std::thread::spawn(move || loop {
            if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(evt) = event::read() {
                    match evt {
                        CrosstermEvent::Key(key) => {
                            if tx_key.send(TuiEvent::Key(key)).is_err() {
                                return;
                            }
                        }
                        CrosstermEvent::Resize(w, h) => {
                            if tx_key.send(TuiEvent::Resize(w, h)).is_err() {
                                return;
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

        // Agent events
        let tx_agent = tx.clone();
        tokio::spawn(async move {
            let mut rx = agent_rx;
            while let Ok(event) = rx.recv().await {
                if tx_agent.send(TuiEvent::Agent(event)).is_err() {
                    return;
                }
            }
        });

        // Tick
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(250));
            loop {
                interval.tick().await;
                if tx.send(TuiEvent::Tick).is_err() {
                    return;
                }
            }
        });

        Self { rx }
    }

    pub async fn next(&mut self) -> Option<TuiEvent> {
        self.rx.recv().await
    }
}
