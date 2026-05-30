use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoCache {
    pub root: PathBuf,
    pub detected_stack: Option<String>,
    pub last_indexed_commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadataCache {
    pub language: String,
    pub package_manager: Option<String>,
    pub entry_points: Vec<String>,
}

impl RepoCache {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            detected_stack: None,
            last_indexed_commit: None,
        }
    }

    pub fn cache_dir(agent_dir: &Path) -> PathBuf {
        agent_dir.join("cache")
    }

    pub async fn load(agent_dir: &Path) -> anyhow::Result<Option<Self>> {
        let path = Self::cache_dir(agent_dir).join("repo_cache.json");
        if !path.exists() {
            return Ok(None);
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let cache: RepoCache = serde_json::from_str(&content)?;
        Ok(Some(cache))
    }

    pub async fn save(&self, agent_dir: &Path) -> anyhow::Result<()> {
        let cache_dir = Self::cache_dir(agent_dir);
        tokio::fs::create_dir_all(&cache_dir).await?;
        let path = cache_dir.join("repo_cache.json");
        let content = serde_json::to_string_pretty(self)?;
        tokio::fs::write(&path, content).await?;
        Ok(())
    }

    /// Check if cache needs refresh by comparing current HEAD with last indexed commit.
    pub async fn needs_refresh(&self, repo_root: &Path) -> bool {
        let Some(ref last_commit) = self.last_indexed_commit else {
            return true;
        };

        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_root)
            .output()
            .await;

        match output {
            Ok(o) => {
                let current = String::from_utf8_lossy(&o.stdout).trim().to_string();
                current != *last_commit
            }
            Err(_) => true,
        }
    }

    /// Update the last indexed commit to current HEAD.
    pub async fn update_commit(&mut self, repo_root: &Path) -> anyhow::Result<()> {
        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_root)
            .output()
            .await?;

        self.last_indexed_commit = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        Ok(())
    }
}
