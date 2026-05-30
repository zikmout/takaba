use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub kind: MemoryKind,
    pub content: String,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    FileContent,
    SearchHit,
    CommandOutput,
    Diff,
    Decision,
}

pub struct ShortTermMemory {
    entries: Vec<MemoryEntry>,
    file_versions: HashMap<String, DateTime<Utc>>,
    max_entries: usize,
}

impl ShortTermMemory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            file_versions: HashMap::new(),
            max_entries,
        }
    }

    pub fn add(&mut self, entry: MemoryEntry) {
        // Deduplicate by source
        self.entries.retain(|e| e.source != entry.source);

        // Track file read timestamps
        if matches!(entry.kind, MemoryKind::FileContent) {
            self.file_versions
                .insert(entry.source.clone(), entry.timestamp);
        }

        self.entries.push(entry);

        // Evict oldest low-priority entries if over limit
        if self.entries.len() > self.max_entries {
            self.entries.sort_by(|a, b| {
                b.priority
                    .cmp(&a.priority)
                    .then(b.timestamp.cmp(&a.timestamp))
            });
            self.entries.truncate(self.max_entries);
        }
    }

    pub fn invalidate_file(&mut self, path: &str) {
        self.entries
            .retain(|e| !(matches!(e.kind, MemoryKind::FileContent) && e.source == path));
        self.file_versions.remove(path);
    }

    pub fn get_recent(&self, limit: usize) -> Vec<&MemoryEntry> {
        let mut sorted: Vec<&MemoryEntry> = self.entries.iter().collect();
        sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        sorted.truncate(limit);
        sorted
    }

    pub fn get_by_kind(&self, kind: &MemoryKind) -> Vec<&MemoryEntry> {
        self.entries
            .iter()
            .filter(|e| std::mem::discriminant(&e.kind) == std::mem::discriminant(kind))
            .collect()
    }

    pub fn estimated_tokens(&self) -> usize {
        self.entries.iter().map(|e| e.content.len() / 4).sum()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.file_versions.clear();
    }
}

impl Default for ShortTermMemory {
    fn default() -> Self {
        Self::new(100)
    }
}
