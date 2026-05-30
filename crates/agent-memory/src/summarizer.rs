use agent_provider::provider::LlmProvider;
use agent_provider::types::{ChatMessage, ChatRequest, Role};

/// Summarize a transcript when it exceeds the token budget.
pub async fn summarize_transcript(
    messages: &[ChatMessage],
    provider: &dyn LlmProvider,
    max_summary_tokens: usize,
) -> anyhow::Result<String> {
    let transcript_text: String = messages
        .iter()
        .map(|m| format!("[{}] {}", role_str(&m.role), m.content))
        .collect::<Vec<_>>()
        .join("\n---\n");

    let request = ChatRequest {
        model: provider.metadata().model,
        messages: vec![
            ChatMessage {
                role: Role::System,
                content: "You are a summarizer. Produce a concise summary of the conversation \
                          focusing on: key decisions made, files modified, errors encountered, \
                          and current state. Keep it under 500 words."
                    .into(),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: Role::User,
                content: format!(
                    "Summarize this agent transcript:\n\n{}",
                    truncate(&transcript_text, max_summary_tokens * 4)
                ),
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        tools: None,
        options: None,
    };

    let response = provider.chat(request).await?;
    Ok(response.message.content)
}

/// Check if summarization is needed based on message count.
pub fn needs_summarization(messages: &[ChatMessage], max_entries: usize) -> bool {
    messages.len() > max_entries
}

fn role_str(role: &Role) -> &str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...[truncated]", &s[..max])
    }
}
