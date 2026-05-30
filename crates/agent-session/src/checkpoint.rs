use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub timestamp: DateTime<Utc>,
    pub iteration: usize,
    pub git_diff: String,
    pub quality_result: Option<QualitySnapshot>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySnapshot {
    pub passed: bool,
    pub summary: String,
}

impl Checkpoint {
    pub fn new(iteration: usize, git_diff: String, description: String) -> Self {
        Self {
            timestamp: Utc::now(),
            iteration,
            git_diff,
            quality_result: None,
            description,
        }
    }

    pub fn with_quality(mut self, passed: bool, summary: String) -> Self {
        self.quality_result = Some(QualitySnapshot { passed, summary });
        self
    }

    /// Save checkpoint to a session directory.
    pub async fn save(&self, session_dir: &Path) -> anyhow::Result<()> {
        let checkpoints_dir = session_dir.join("checkpoints");
        tokio::fs::create_dir_all(&checkpoints_dir).await?;

        let filename = format!("checkpoint_{:03}.json", self.iteration);
        let path = checkpoints_dir.join(filename);
        let content = serde_json::to_string_pretty(self)?;
        tokio::fs::write(&path, content).await?;

        Ok(())
    }

    /// Load a checkpoint from file.
    pub async fn load(path: &Path) -> anyhow::Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let checkpoint: Checkpoint = serde_json::from_str(&content)?;
        Ok(checkpoint)
    }

    /// List all checkpoints in a session directory.
    pub async fn list(session_dir: &Path) -> anyhow::Result<Vec<Checkpoint>> {
        let checkpoints_dir = session_dir.join("checkpoints");
        if !checkpoints_dir.exists() {
            return Ok(Vec::new());
        }

        let mut checkpoints = Vec::new();
        let mut entries = tokio::fs::read_dir(&checkpoints_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(cp) = Checkpoint::load(&path).await {
                    checkpoints.push(cp);
                }
            }
        }

        checkpoints.sort_by_key(|c| c.iteration);
        Ok(checkpoints)
    }
}
