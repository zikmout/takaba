use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("provider error: {0}")]
    Provider(String),

    #[error("tool error: {tool}: {message}")]
    Tool { tool: String, message: String },

    #[error("policy denied: {0}")]
    PolicyDenied(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("session error: {0}")]
    Session(String),

    #[error("max iterations reached: {0}")]
    MaxIterations(usize),

    #[error("context overflow: used {used} tokens, max {max}")]
    ContextOverflow { used: usize, max: usize },

    #[error("parse error: {0}")]
    Parse(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type AgentResult<T> = Result<T, AgentError>;
