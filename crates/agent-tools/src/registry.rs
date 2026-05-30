use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolEntry {
    pub server: String,
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum RegisteredTool {
    Local(LocalToolDefinition),
    Mcp(McpToolEntry),
}

impl RegisteredTool {
    pub fn name(&self) -> &str {
        match self {
            Self::Local(t) => &t.name,
            Self::Mcp(t) => &t.name,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::Local(t) => &t.description,
            Self::Mcp(t) => &t.description,
        }
    }

    pub fn parameters(&self) -> &serde_json::Value {
        match self {
            Self::Local(t) => &t.parameters,
            Self::Mcp(t) => &t.parameters,
        }
    }

    /// Convert to a tool definition JSON suitable for the LLM payload.
    pub fn to_tool_definition(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": self.description(),
                "parameters": self.parameters(),
            }
        })
    }
}

pub struct ToolRegistry {
    tools: HashMap<String, RegisteredTool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register_local(&mut self, tool: LocalToolDefinition) {
        self.tools
            .insert(tool.name.clone(), RegisteredTool::Local(tool));
    }

    pub fn register_mcp(&mut self, server: &str, tool: McpToolEntry) {
        let key = format!("mcp.{}.{}", server, tool.name);
        self.tools.insert(key, RegisteredTool::Mcp(tool));
    }

    pub fn get_tool(&self, name: &str) -> Option<&RegisteredTool> {
        self.tools.get(name)
    }

    pub fn list_tools(&self) -> Vec<&RegisteredTool> {
        self.tools.values().collect()
    }

    pub fn tool_definitions(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|t| t.to_tool_definition())
            .collect()
    }

    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a default registry with all built-in local tools registered.
pub fn create_default_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    let tools = vec![
        LocalToolDefinition {
            name: "rg_search".into(),
            description: "Search for a pattern in the codebase using ripgrep".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Regex pattern to search for" },
                    "glob": { "type": "string", "description": "File glob filter" },
                    "case_sensitive": { "type": "boolean", "default": false },
                    "max_results": { "type": "integer", "default": 50 },
                    "context_lines": { "type": "integer", "default": 2 }
                },
                "required": ["pattern"]
            }),
        },
        LocalToolDefinition {
            name: "file_read".into(),
            description: "Read a file's content with optional line range".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path relative to repo root" },
                    "start_line": { "type": "integer" },
                    "end_line": { "type": "integer" }
                },
                "required": ["path"]
            }),
        },
        LocalToolDefinition {
            name: "file_write".into(),
            description: "Write content to a file".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" },
                    "create_dirs": { "type": "boolean", "default": false }
                },
                "required": ["path", "content"]
            }),
        },
        LocalToolDefinition {
            name: "repo_tree".into(),
            description: "List repository file tree".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "max_depth": { "type": "integer", "default": 3 },
                    "path": { "type": "string" }
                }
            }),
        },
        LocalToolDefinition {
            name: "git_status".into(),
            description: "Show git status".into(),
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        },
        LocalToolDefinition {
            name: "git_diff".into(),
            description: "Show git diff".into(),
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        },
        LocalToolDefinition {
            name: "git_commit".into(),
            description: "Create a git commit".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" },
                    "files": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["message", "files"]
            }),
        },
        LocalToolDefinition {
            name: "git_branch".into(),
            description: "Create and checkout a new git branch".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"]
            }),
        },
        LocalToolDefinition {
            name: "run_command".into(),
            description: "Run an allowed shell command".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" },
                    "args": { "type": "array", "items": { "type": "string" } },
                    "timeout_secs": { "type": "integer", "default": 30 }
                },
                "required": ["command", "args"]
            }),
        },
        LocalToolDefinition {
            name: "apply_patch".into(),
            description: "Apply a unified diff patch".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "diff": { "type": "string" },
                    "dry_run": { "type": "boolean", "default": false }
                },
                "required": ["diff"]
            }),
        },
        LocalToolDefinition {
            name: "detect_project".into(),
            description: "Detect project type, language, and available commands".into(),
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        },
        LocalToolDefinition {
            name: "run_quality_gate".into(),
            description: "Run quality checks (fmt, lint, test)".into(),
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        },
        LocalToolDefinition {
            name: "symbol_index".into(),
            description: "Index symbols (functions, structs, classes) in the codebase".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "glob": { "type": "string" }
                }
            }),
        },
    ];

    for tool in tools {
        registry.register_local(tool);
    }

    registry
}
