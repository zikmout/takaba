use agent_provider::types::ChatMessage;
use agent_tools::types::ProjectMetadata;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub kind: ContextKind,
    pub content: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextKind {
    FileContent,
    SearchResult,
    CommandOutput,
    GitDiff,
    QualityResult,
    ProjectInfo,
    SymbolIndex,
}

#[derive(Debug, Default)]
pub struct AgentState {
    pub messages: Vec<ChatMessage>,
    pub context_entries: Vec<ContextEntry>,
    pub project: Option<ProjectMetadata>,
    pub iteration: usize,
    pub files_read: Vec<String>,
    pub files_modified: Vec<String>,
    pub last_diff: Option<String>,
}

impl AgentState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_context(&mut self, entry: ContextEntry) {
        // Deduplicate by source
        if !self
            .context_entries
            .iter()
            .any(|e| e.source == entry.source && e.kind_matches(&entry))
        {
            self.context_entries.push(entry);
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    pub fn record_file_read(&mut self, path: &str) {
        if !self.files_read.contains(&path.to_string()) {
            self.files_read.push(path.to_string());
        }
    }

    pub fn record_file_modified(&mut self, path: &str) {
        if !self.files_modified.contains(&path.to_string()) {
            self.files_modified.push(path.to_string());
        }
    }

    pub fn estimated_tokens(&self) -> usize {
        // Rough estimate: 4 chars per token
        let message_chars: usize = self.messages.iter().map(|m| m.content.len()).sum();
        let context_chars: usize = self.context_entries.iter().map(|e| e.content.len()).sum();
        (message_chars + context_chars) / 4
    }
}

impl ContextEntry {
    fn kind_matches(&self, other: &ContextEntry) -> bool {
        matches!(
            (&self.kind, &other.kind),
            (ContextKind::FileContent, ContextKind::FileContent)
                | (ContextKind::SearchResult, ContextKind::SearchResult)
                | (ContextKind::CommandOutput, ContextKind::CommandOutput)
                | (ContextKind::GitDiff, ContextKind::GitDiff)
                | (ContextKind::QualityResult, ContextKind::QualityResult)
                | (ContextKind::ProjectInfo, ContextKind::ProjectInfo)
                | (ContextKind::SymbolIndex, ContextKind::SymbolIndex)
        )
    }
}
