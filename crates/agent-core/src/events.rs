use agent_session::session::SessionId;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::types::ToolCallId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    SessionStarted(SessionId),
    IterationStarted(usize),
    ToolCallStarted(ToolCallId, String),
    ToolCallFinished(ToolCallId, String),
    ModelToken(String),
    ModelResponseComplete(String),
    PatchProposed(String),
    PatchApplied(String),
    QualityGateStarted,
    QualityGateFinished(bool),
    SessionCompleted,
    Error(String),
}

pub struct EventBus {
    sender: broadcast::Sender<AgentEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn emit(&self, event: AgentEvent) {
        // Ignore error if no receivers
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(256)
    }
}
