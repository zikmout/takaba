use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolEntry {
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolIndex {
    pub symbols: Vec<SymbolEntry>,
    pub indexed_commit: Option<String>,
}

impl SymbolIndex {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
            indexed_commit: None,
        }
    }

    fn cache_path(agent_dir: &Path) -> PathBuf {
        agent_dir.join("cache").join("symbols.json")
    }

    pub async fn load(agent_dir: &Path) -> anyhow::Result<Option<Self>> {
        let path = Self::cache_path(agent_dir);
        if !path.exists() {
            return Ok(None);
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let index: SymbolIndex = serde_json::from_str(&content)?;
        Ok(Some(index))
    }

    pub async fn save(&self, agent_dir: &Path) -> anyhow::Result<()> {
        let cache_dir = agent_dir.join("cache");
        tokio::fs::create_dir_all(&cache_dir).await?;
        let path = Self::cache_path(agent_dir);
        let content = serde_json::to_string_pretty(self)?;
        tokio::fs::write(&path, content).await?;
        Ok(())
    }

    /// Check if refresh is needed by comparing commits.
    pub async fn needs_refresh(&self, repo_root: &Path) -> bool {
        let Some(ref indexed) = self.indexed_commit else {
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
                current != *indexed
            }
            Err(_) => true,
        }
    }

    pub fn search(&self, query: &str) -> Vec<&SymbolEntry> {
        let query_lower = query.to_lowercase();
        self.symbols
            .iter()
            .filter(|s| s.name.to_lowercase().contains(&query_lower))
            .collect()
    }
}

impl Default for SymbolIndex {
    fn default() -> Self {
        Self::new()
    }
}
