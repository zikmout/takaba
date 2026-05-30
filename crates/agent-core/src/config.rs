use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default)]
    pub provider: ProviderConfig,
    #[serde(default)]
    pub agent: AgentSettings,
    #[serde(default)]
    pub git: GitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default = "default_provider_type")]
    pub r#type: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_model")]
    pub model: String,
}

fn default_provider_type() -> String {
    "ollama".into()
}
fn default_base_url() -> String {
    "http://localhost:11434".into()
}
fn default_model() -> String {
    "code-agent-14b".into()
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            r#type: default_provider_type(),
            base_url: default_base_url(),
            model: default_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSettings {
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    #[serde(default = "default_max_context_tokens")]
    pub max_context_tokens: usize,
    #[serde(default = "default_max_files")]
    pub max_files_per_iteration: usize,
    #[serde(default = "default_max_file_size")]
    pub max_file_size_kb: usize,
}

fn default_max_iterations() -> usize {
    5
}
fn default_max_context_tokens() -> usize {
    12000
}
fn default_max_files() -> usize {
    12
}
fn default_max_file_size() -> usize {
    200
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            max_iterations: default_max_iterations(),
            max_context_tokens: default_max_context_tokens(),
            max_files_per_iteration: default_max_files(),
            max_file_size_kb: default_max_file_size(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    #[serde(default = "default_true")]
    pub auto_branch: bool,
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,
}

fn default_true() -> bool {
    true
}
fn default_branch_prefix() -> String {
    "agent/".into()
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            auto_branch: default_true(),
            branch_prefix: default_branch_prefix(),
        }
    }
}

impl AgentConfig {
    pub fn load(working_dir: &Path) -> anyhow::Result<Self> {
        let config_path = working_dir.join(".agent").join("config.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let mut config: AgentConfig = toml::from_str(&content)?;
            config.apply_env_overrides();
            Ok(config)
        } else {
            let mut config = AgentConfig::default();
            config.apply_env_overrides();
            Ok(config)
        }
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(url) = std::env::var("AGENT_PROVIDER_URL") {
            self.provider.base_url = url;
        }
        if let Ok(model) = std::env::var("AGENT_MODEL") {
            self.provider.model = model;
        }
        if let Ok(provider_type) = std::env::var("AGENT_PROVIDER_TYPE") {
            self.provider.r#type = provider_type;
        }
    }

    pub fn agent_dir(working_dir: &Path) -> PathBuf {
        working_dir.join(".agent")
    }
}
