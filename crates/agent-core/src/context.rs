use agent_provider::types::{ChatMessage, Role};

use crate::config::AgentConfig;
use crate::state::{AgentState, ContextKind};
use crate::types::AgentMode;

/// Build the context pack (system prompt + messages) for the model.
pub fn build_context_pack(
    state: &AgentState,
    task_prompt: &str,
    mode: AgentMode,
    config: &AgentConfig,
    tool_definitions: &[serde_json::Value],
) -> Vec<ChatMessage> {
    let mut messages = Vec::new();

    // System prompt
    let system = build_system_prompt(state, mode, config, tool_definitions);
    messages.push(ChatMessage {
        role: Role::System,
        content: system,
        tool_calls: None,
        tool_call_id: None,
    });

    // User task
    messages.push(ChatMessage {
        role: Role::User,
        content: task_prompt.to_string(),
        tool_calls: None,
        tool_call_id: None,
    });

    // Append conversation history (capped)
    let max_tokens = config.agent.max_context_tokens;
    let mut token_budget = max_tokens.saturating_sub(estimate_tokens(&messages));

    for msg in &state.messages {
        let msg_tokens = msg.content.len() / 4;
        if msg_tokens > token_budget {
            break;
        }
        token_budget -= msg_tokens;
        messages.push(msg.clone());
    }

    messages
}

fn build_system_prompt(
    state: &AgentState,
    mode: AgentMode,
    _config: &AgentConfig,
    tool_definitions: &[serde_json::Value],
) -> String {
    let mut parts = Vec::new();

    parts.push(format!(
        "You are a code agent operating in '{}' mode.",
        mode
    ));

    match mode {
        AgentMode::Ask => {
            parts.push("You can only read files and search code. Do not modify anything.".into());
        }
        AgentMode::Plan => {
            parts.push(
                "Analyze the codebase and produce a detailed plan. Do not modify files.".into(),
            );
        }
        AgentMode::Edit => {
            parts.push("You may read, search, and modify files. Apply patches carefully.".into());
        }
        AgentMode::Auto => {
            parts.push(
                "Full autonomous mode. Read, search, modify, run commands, and commit.".into(),
            );
        }
    }

    // Project info
    if let Some(ref project) = state.project {
        parts.push(format!(
            "\nProject: {} ({})",
            project.language,
            project.package_manager.as_deref().unwrap_or("unknown")
        ));
    }

    // Context entries summary
    let file_count = state
        .context_entries
        .iter()
        .filter(|e| matches!(e.kind, ContextKind::FileContent))
        .count();
    let search_count = state
        .context_entries
        .iter()
        .filter(|e| matches!(e.kind, ContextKind::SearchResult))
        .count();

    if file_count > 0 || search_count > 0 {
        parts.push(format!(
            "\nContext loaded: {file_count} files, {search_count} search results"
        ));
    }

    // Files read
    if !state.files_read.is_empty() {
        parts.push(format!(
            "\nFiles already read: {}",
            state.files_read.join(", ")
        ));
    }

    // Available tools
    if !tool_definitions.is_empty() {
        parts.push(format!(
            "\nYou have {} tools available.",
            tool_definitions.len()
        ));
    }

    // Response format
    parts.push("\nRespond with a JSON object containing your action:".into());
    parts.push(r#"- {"type": "answer", "content": "..."} to respond"#.into());
    parts.push(
        r#"- {"type": "tool_call", "id": "...", "name": "tool_name", "arguments": {...}} to call a tool"#
            .into(),
    );
    parts.push(
        r#"- {"type": "need_more_context", "queries": ["..."]} to request more information"#.into(),
    );

    parts.join("\n")
}

fn estimate_tokens(messages: &[ChatMessage]) -> usize {
    messages.iter().map(|m| m.content.len() / 4).sum()
}
