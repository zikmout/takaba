use chrono::Utc;
use std::path::{Path, PathBuf};

use crate::session::{AgentSession, SessionId};

pub struct SessionStore {
    base_dir: PathBuf,
}

impl SessionStore {
    pub fn new(agent_dir: &Path) -> Self {
        Self {
            base_dir: agent_dir.join("sessions"),
        }
    }

    /// Create a new session directory and persist session metadata.
    pub async fn create(&self, session: &AgentSession) -> anyhow::Result<PathBuf> {
        let now = Utc::now();
        let slug = slugify(&session.prompt);
        let dir_name = format!(
            "{}_{}-{}_{slug}",
            now.format("%Y-%m-%d"),
            now.format("%H"),
            now.format("%M"),
        );

        let session_dir = self.base_dir.join(&dir_name);
        tokio::fs::create_dir_all(&session_dir).await?;

        let meta_path = session_dir.join("session.json");
        let meta = serde_json::to_string_pretty(session)?;
        tokio::fs::write(&meta_path, meta).await?;

        // Update latest symlink
        let latest = self.base_dir.join("latest");
        let _ = tokio::fs::remove_file(&latest).await;
        #[cfg(unix)]
        {
            tokio::fs::symlink(&session_dir, &latest).await?;
        }

        Ok(session_dir)
    }

    /// Load a session by scanning session directories for a matching ID.
    pub async fn load(&self, id: &SessionId) -> anyhow::Result<Option<AgentSession>> {
        let mut entries = tokio::fs::read_dir(&self.base_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let meta_path = path.join("session.json");
            if !meta_path.exists() {
                continue;
            }
            let content = tokio::fs::read_to_string(&meta_path).await?;
            if let Ok(session) = serde_json::from_str::<AgentSession>(&content) {
                if session.id == *id {
                    return Ok(Some(session));
                }
            }
        }

        Ok(None)
    }

    /// List all sessions.
    pub async fn list(&self) -> anyhow::Result<Vec<AgentSession>> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.base_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let meta_path = path.join("session.json");
            if !meta_path.exists() {
                continue;
            }
            let content = tokio::fs::read_to_string(&meta_path).await?;
            if let Ok(session) = serde_json::from_str::<AgentSession>(&content) {
                sessions.push(session);
            }
        }

        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(sessions)
    }

    /// Update session metadata on disk.
    pub async fn update(&self, session: &AgentSession) -> anyhow::Result<()> {
        let mut entries = tokio::fs::read_dir(&self.base_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let meta_path = path.join("session.json");
            if !meta_path.exists() {
                continue;
            }
            let content = tokio::fs::read_to_string(&meta_path).await?;
            if let Ok(existing) = serde_json::from_str::<AgentSession>(&content) {
                if existing.id == session.id {
                    let meta = serde_json::to_string_pretty(session)?;
                    tokio::fs::write(&meta_path, meta).await?;
                    return Ok(());
                }
            }
        }

        anyhow::bail!("session {} not found", session.id)
    }

    pub fn transcript_path(&self, session_dir: &Path) -> PathBuf {
        session_dir.join("transcript.jsonl")
    }
}

fn slugify(text: &str) -> String {
    text.chars()
        .take(30)
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}
