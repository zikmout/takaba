use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub timestamp: DateTime<Utc>,
    pub kind: TranscriptKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TranscriptKind {
    UserMessage {
        content: String,
    },
    ModelResponse {
        content: String,
    },
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    ToolResult {
        id: String,
        name: String,
        success: bool,
        output: String,
    },
    Error {
        message: String,
    },
}

impl TranscriptEntry {
    pub fn user_message(content: String) -> Self {
        Self {
            timestamp: Utc::now(),
            kind: TranscriptKind::UserMessage { content },
        }
    }

    pub fn model_response(content: String) -> Self {
        Self {
            timestamp: Utc::now(),
            kind: TranscriptKind::ModelResponse { content },
        }
    }

    pub fn tool_call(id: String, name: String, arguments: serde_json::Value) -> Self {
        Self {
            timestamp: Utc::now(),
            kind: TranscriptKind::ToolCall {
                id,
                name,
                arguments,
            },
        }
    }

    pub fn tool_result(id: String, name: String, success: bool, output: String) -> Self {
        Self {
            timestamp: Utc::now(),
            kind: TranscriptKind::ToolResult {
                id,
                name,
                success,
                output,
            },
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            timestamp: Utc::now(),
            kind: TranscriptKind::Error { message },
        }
    }
}

/// Append a transcript entry to a JSONL file.
pub async fn append_entry(path: &Path, entry: &TranscriptEntry) -> anyhow::Result<()> {
    let mut line = serde_json::to_string(entry)?;
    line.push('\n');

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;

    file.write_all(line.as_bytes()).await?;
    Ok(())
}

/// Read all transcript entries from a JSONL file.
pub async fn read_entries(path: &Path) -> anyhow::Result<Vec<TranscriptEntry>> {
    let file = tokio::fs::File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut entries = Vec::new();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<TranscriptEntry>(&line) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                tracing::warn!("skipping malformed transcript line: {e}");
            }
        }
    }

    Ok(entries)
}
