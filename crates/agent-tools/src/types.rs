use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgSearchInput {
    pub pattern: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glob: Option<String>,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default = "default_context_lines")]
    pub context_lines: usize,
}

fn default_max_results() -> usize {
    50
}
fn default_context_lines() -> usize {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgSearchOutput {
    pub matches: Vec<RgMatch>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgMatch {
    pub path: String,
    pub line_number: usize,
    pub line_content: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadInput {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadOutput {
    pub path: String,
    pub content: String,
    pub total_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWriteInput {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub create_dirs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWriteOutput {
    pub path: String,
    pub bytes_written: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoTreeInput {
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

fn default_max_depth() -> usize {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoTreeOutput {
    pub entries: Vec<TreeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    pub path: String,
    pub is_dir: bool,
    pub depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatusOutput {
    pub branch: String,
    pub files: Vec<GitFileStatus>,
    pub clean: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFileStatus {
    pub path: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffOutput {
    pub diff: String,
    pub staged_diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommitInput {
    pub message: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommitOutput {
    pub commit_hash: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommandInput {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_timeout_secs() -> u64 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchInput {
    pub diff: String,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchOutput {
    pub applied: bool,
    pub files_changed: Vec<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub language: String,
    pub package_manager: Option<String>,
    pub test_command: Option<String>,
    pub lint_command: Option<String>,
    pub typecheck_command: Option<String>,
    pub build_command: Option<String>,
    pub root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGateResult {
    pub passed: bool,
    pub steps: Vec<QualityStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityStep {
    pub name: String,
    pub passed: bool,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Class,
    Interface,
    Const,
    Type,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
}
