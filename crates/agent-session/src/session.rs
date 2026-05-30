use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Completed,
    Failed,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: SessionId,
    pub status: SessionStatus,
    pub mode: String,
    pub prompt: String,
    pub working_dir: std::path::PathBuf,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub iteration_count: usize,
}

impl AgentSession {
    pub fn new(prompt: String, mode: String, working_dir: std::path::PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            status: SessionStatus::Active,
            mode,
            prompt,
            working_dir,
            created_at: now,
            updated_at: now,
            iteration_count: 0,
        }
    }
}
