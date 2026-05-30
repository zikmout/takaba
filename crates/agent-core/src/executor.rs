use std::path::Path;

use agent_tools::types::ToolResult;

use crate::events::{AgentEvent, EventBus};
use crate::policy::{PolicyResult, ToolPolicy};
use crate::types::{AgentMode, ToolCallId};

/// Execute a tool call after checking policy.
pub async fn execute_tool_call(
    id: &ToolCallId,
    name: &str,
    arguments: &serde_json::Value,
    repo_root: &Path,
    policy: &dyn ToolPolicy,
    mode: AgentMode,
    event_bus: &EventBus,
) -> ToolResult {
    // Check policy
    match policy.check(name, arguments, mode).await {
        PolicyResult::Allow => {}
        PolicyResult::Deny(reason) => {
            return ToolResult {
                tool_name: name.to_string(),
                success: false,
                output: serde_json::Value::Null,
                error: Some(format!("policy denied: {reason}")),
            };
        }
        PolicyResult::RequireConfirmation(reason) => {
            tracing::warn!("tool requires confirmation: {reason}");
            // In a full implementation, this would prompt the user
            // For now, we allow it with a warning
        }
    }

    event_bus.emit(AgentEvent::ToolCallStarted(id.clone(), name.to_string()));

    let result = dispatch_tool(name, arguments, repo_root).await;

    event_bus.emit(AgentEvent::ToolCallFinished(id.clone(), name.to_string()));

    result
}

async fn dispatch_tool(name: &str, arguments: &serde_json::Value, repo_root: &Path) -> ToolResult {
    let result = match name {
        "rg_search" => {
            let input: agent_tools::types::RgSearchInput =
                match serde_json::from_value(arguments.clone()) {
                    Ok(v) => v,
                    Err(e) => return tool_error(name, &e.to_string()),
                };
            match agent_tools::rg::rg_search(&input, repo_root).await {
                Ok(output) => tool_ok(name, &output),
                Err(e) => tool_error(name, &e.to_string()),
            }
        }
        "file_read" => {
            let input: agent_tools::types::FileReadInput =
                match serde_json::from_value(arguments.clone()) {
                    Ok(v) => v,
                    Err(e) => return tool_error(name, &e.to_string()),
                };
            match agent_tools::fs::file_read(&input, repo_root).await {
                Ok(output) => tool_ok(name, &output),
                Err(e) => tool_error(name, &e.to_string()),
            }
        }
        "file_write" => {
            let input: agent_tools::types::FileWriteInput =
                match serde_json::from_value(arguments.clone()) {
                    Ok(v) => v,
                    Err(e) => return tool_error(name, &e.to_string()),
                };
            match agent_tools::fs::file_write(&input, repo_root).await {
                Ok(output) => tool_ok(name, &output),
                Err(e) => tool_error(name, &e.to_string()),
            }
        }
        "repo_tree" => {
            let input: agent_tools::types::RepoTreeInput =
                match serde_json::from_value(arguments.clone()) {
                    Ok(v) => v,
                    Err(e) => return tool_error(name, &e.to_string()),
                };
            match agent_tools::fs::repo_tree(&input, repo_root) {
                Ok(output) => tool_ok(name, &output),
                Err(e) => tool_error(name, &e.to_string()),
            }
        }
        "git_status" => match agent_tools::git::git_status(repo_root).await {
            Ok(output) => tool_ok(name, &output),
            Err(e) => tool_error(name, &e.to_string()),
        },
        "git_diff" => match agent_tools::git::git_diff(repo_root).await {
            Ok(output) => tool_ok(name, &output),
            Err(e) => tool_error(name, &e.to_string()),
        },
        "git_commit" => {
            let input: agent_tools::types::GitCommitInput =
                match serde_json::from_value(arguments.clone()) {
                    Ok(v) => v,
                    Err(e) => return tool_error(name, &e.to_string()),
                };
            match agent_tools::git::git_commit(&input, repo_root).await {
                Ok(output) => tool_ok(name, &output),
                Err(e) => tool_error(name, &e.to_string()),
            }
        }
        "git_branch" => {
            let branch_name = arguments
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("agent/unnamed");
            match agent_tools::git::git_branch(branch_name, repo_root).await {
                Ok(output) => tool_ok(name, &output),
                Err(e) => tool_error(name, &e.to_string()),
            }
        }
        "run_command" => {
            let input: agent_tools::types::ShellCommandInput =
                match serde_json::from_value(arguments.clone()) {
                    Ok(v) => v,
                    Err(e) => return tool_error(name, &e.to_string()),
                };
            match agent_tools::shell::run_command(&input, repo_root).await {
                Ok(output) => tool_ok(name, &output),
                Err(e) => tool_error(name, &e.to_string()),
            }
        }
        "apply_patch" => {
            let input: agent_tools::types::PatchInput =
                match serde_json::from_value(arguments.clone()) {
                    Ok(v) => v,
                    Err(e) => return tool_error(name, &e.to_string()),
                };
            match agent_tools::patch::apply_patch(&input, repo_root).await {
                Ok(output) => tool_ok(name, &output),
                Err(e) => tool_error(name, &e.to_string()),
            }
        }
        "detect_project" => match agent_tools::project::detect_project(repo_root) {
            Ok(output) => tool_ok(name, &output),
            Err(e) => tool_error(name, &e.to_string()),
        },
        "run_quality_gate" => {
            let project = match agent_tools::project::detect_project(repo_root) {
                Ok(p) => p,
                Err(e) => return tool_error(name, &e.to_string()),
            };
            match agent_tools::quality::run_quality_gate(&project, repo_root).await {
                Ok(output) => tool_ok(name, &output),
                Err(e) => tool_error(name, &e.to_string()),
            }
        }
        "symbol_index" => {
            let glob = arguments
                .get("glob")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            match agent_tools::symbol::symbol_index(repo_root, glob.as_deref()).await {
                Ok(output) => tool_ok(name, &output),
                Err(e) => tool_error(name, &e.to_string()),
            }
        }
        _ => tool_error(name, &format!("unknown tool: {name}")),
    };

    result
}

fn tool_ok<T: serde::Serialize>(name: &str, output: &T) -> ToolResult {
    ToolResult {
        tool_name: name.to_string(),
        success: true,
        output: serde_json::to_value(output).unwrap_or_default(),
        error: None,
    }
}

fn tool_error(name: &str, error: &str) -> ToolResult {
    ToolResult {
        tool_name: name.to_string(),
        success: false,
        output: serde_json::Value::Null,
        error: Some(error.to_string()),
    }
}
