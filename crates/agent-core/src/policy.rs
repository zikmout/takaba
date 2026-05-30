use async_trait::async_trait;

use crate::types::AgentMode;

#[async_trait]
pub trait ToolPolicy: Send + Sync {
    /// Check if a tool call is allowed.
    async fn check(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
        mode: AgentMode,
    ) -> PolicyResult;
}

#[derive(Debug, Clone)]
pub enum PolicyResult {
    Allow,
    Deny(String),
    RequireConfirmation(String),
}

pub struct DefaultPolicy;

impl DefaultPolicy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultPolicy {
    fn default() -> Self {
        Self::new()
    }
}

const SENSITIVE_FILES: &[&str] = &[
    ".env",
    ".pem",
    ".key",
    "id_rsa",
    "id_ed25519",
    "terraform.tfstate",
    ".secret",
    "credentials",
    "password",
];

#[async_trait]
impl ToolPolicy for DefaultPolicy {
    async fn check(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
        mode: AgentMode,
    ) -> PolicyResult {
        // In Ask mode, block all write operations
        if mode == AgentMode::Ask {
            match tool_name {
                "file_write" | "apply_patch" | "git_commit" | "git_branch" | "run_command" => {
                    return PolicyResult::Deny(format!(
                        "tool '{tool_name}' not allowed in ask mode"
                    ));
                }
                _ => {}
            }
        }

        // In Plan mode, block writes
        if mode == AgentMode::Plan {
            match tool_name {
                "file_write" | "apply_patch" | "git_commit" | "git_branch" => {
                    return PolicyResult::Deny(format!(
                        "tool '{tool_name}' not allowed in plan mode"
                    ));
                }
                _ => {}
            }
        }

        // Never allow push
        if tool_name == "run_command" {
            if let Some(args) = arguments.get("args").and_then(|a| a.as_array()) {
                let args_str: Vec<&str> = args.iter().filter_map(|a| a.as_str()).collect();
                let full = args_str.join(" ");
                if full.contains("push") || full.contains("force") {
                    return PolicyResult::Deny("push/force operations are not allowed".into());
                }
            }
        }

        // Check for sensitive file access in write tools
        if matches!(tool_name, "file_write" | "apply_patch") {
            if let Some(path) = arguments
                .get("path")
                .or_else(|| arguments.get("diff"))
                .and_then(|v| v.as_str())
            {
                for pattern in SENSITIVE_FILES {
                    if path.contains(pattern) {
                        return PolicyResult::Deny(format!(
                            "access to sensitive file pattern '{pattern}' denied"
                        ));
                    }
                }
            }
        }

        // In Edit mode, require confirmation for commits
        if mode == AgentMode::Edit && tool_name == "git_commit" {
            return PolicyResult::RequireConfirmation("confirm git commit".into());
        }

        PolicyResult::Allow
    }
}
