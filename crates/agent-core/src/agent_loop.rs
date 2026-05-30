use std::path::PathBuf;
use std::sync::Arc;

use agent_provider::provider::LlmProvider;
use agent_provider::types::{ChatMessage, ChatRequest, Role};
use agent_session::checkpoint::Checkpoint;
use agent_session::session::AgentSession;
use agent_session::store::SessionStore;
use agent_session::transcript::TranscriptEntry;
use agent_tools::registry::ToolRegistry;
use futures::StreamExt;

use crate::config::AgentConfig;
use crate::context::build_context_pack;
use crate::events::{AgentEvent, EventBus};
use crate::executor::execute_tool_call;
use crate::policy::DefaultPolicy;
use crate::state::{AgentState, ContextEntry, ContextKind};
use crate::types::{AgentAction, AgentTask};

pub struct AgentLoop {
    config: AgentConfig,
    provider: Arc<dyn LlmProvider>,
    registry: ToolRegistry,
    event_bus: Arc<EventBus>,
    working_dir: PathBuf,
}

impl AgentLoop {
    pub fn new(
        config: AgentConfig,
        provider: Arc<dyn LlmProvider>,
        registry: ToolRegistry,
        event_bus: Arc<EventBus>,
        working_dir: PathBuf,
    ) -> Self {
        Self {
            config,
            provider,
            registry,
            event_bus,
            working_dir,
        }
    }

    pub async fn run(&self, task: AgentTask) -> anyhow::Result<String> {
        let agent_dir = AgentConfig::agent_dir(&self.working_dir);
        let store = SessionStore::new(&agent_dir);

        // Create session
        let session = AgentSession::new(
            task.prompt.clone(),
            task.mode.to_string(),
            self.working_dir.clone(),
        );
        let session_dir = store.create(&session).await?;

        self.event_bus
            .emit(AgentEvent::SessionStarted(session.id.clone()));

        let mut state = AgentState::new();

        // Detect project
        if let Ok(project) = agent_tools::project::detect_project(&self.working_dir) {
            state.add_context(ContextEntry {
                kind: ContextKind::ProjectInfo,
                content: serde_json::to_string_pretty(&project).unwrap_or_default(),
                source: "detect_project".into(),
            });
            state.project = Some(project);
        }

        // Get git status
        if let Ok(git_status) = agent_tools::git::git_status(&self.working_dir).await {
            state.add_context(ContextEntry {
                kind: ContextKind::GitDiff,
                content: serde_json::to_string_pretty(&git_status).unwrap_or_default(),
                source: "git_status".into(),
            });
        }

        let policy = DefaultPolicy::new();
        let tool_defs = self.registry.tool_definitions();
        let transcript_path = store.transcript_path(&session_dir);
        let mut final_answer = String::new();

        // Main iteration loop
        for iteration in 0..self.config.agent.max_iterations {
            state.iteration = iteration;
            self.event_bus.emit(AgentEvent::IterationStarted(iteration));

            // Build context
            let messages =
                build_context_pack(&state, &task.prompt, task.mode, &self.config, &tool_defs);

            // Build request
            let request = ChatRequest {
                model: self.config.provider.model.clone(),
                messages,
                tools: if tool_defs.is_empty() {
                    None
                } else {
                    Some(
                        tool_defs
                            .iter()
                            .map(|d| serde_json::from_value(d.clone()).unwrap())
                            .collect(),
                    )
                },
                options: None,
            };

            // Stream response
            let mut response_content = String::new();

            match self.provider.chat_stream(request).await {
                Ok(mut stream) => {
                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(chunk) => {
                                let token = &chunk.message.content;
                                if !token.is_empty() {
                                    self.event_bus.emit(AgentEvent::ModelToken(token.clone()));
                                    response_content.push_str(token);
                                }
                                if chunk.done {
                                    break;
                                }
                            }
                            Err(e) => {
                                self.event_bus
                                    .emit(AgentEvent::Error(format!("stream error: {e}")));
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    // Fallback to non-streaming
                    let request = ChatRequest {
                        model: self.config.provider.model.clone(),
                        messages: build_context_pack(
                            &state,
                            &task.prompt,
                            task.mode,
                            &self.config,
                            &tool_defs,
                        ),
                        tools: None,
                        options: None,
                    };

                    match self.provider.chat(request).await {
                        Ok(response) => {
                            response_content = response.message.content;
                        }
                        Err(e2) => {
                            self.event_bus.emit(AgentEvent::Error(format!(
                                "provider error: stream={e}, fallback={e2}"
                            )));
                            break;
                        }
                    }
                }
            }

            self.event_bus
                .emit(AgentEvent::ModelResponseComplete(response_content.clone()));

            // Write transcript
            let _ = agent_session::transcript::append_entry(
                &transcript_path,
                &TranscriptEntry::model_response(response_content.clone()),
            )
            .await;

            // Parse action
            let action = parse_action(&response_content);

            match action {
                AgentAction::Answer { content } => {
                    final_answer = content;
                    break;
                }
                AgentAction::ToolCall {
                    id,
                    name,
                    arguments,
                } => {
                    let _ = agent_session::transcript::append_entry(
                        &transcript_path,
                        &TranscriptEntry::tool_call(id.0.clone(), name.clone(), arguments.clone()),
                    )
                    .await;

                    let result = execute_tool_call(
                        &id,
                        &name,
                        &arguments,
                        &self.working_dir,
                        &policy,
                        task.mode,
                        &self.event_bus,
                    )
                    .await;

                    let _ = agent_session::transcript::append_entry(
                        &transcript_path,
                        &TranscriptEntry::tool_result(
                            id.0.clone(),
                            name.clone(),
                            result.success,
                            serde_json::to_string(&result.output).unwrap_or_default(),
                        ),
                    )
                    .await;

                    // Add result to state
                    let result_msg = if result.success {
                        serde_json::to_string_pretty(&result.output).unwrap_or_default()
                    } else {
                        result.error.unwrap_or_default()
                    };

                    state.add_message(ChatMessage {
                        role: Role::Assistant,
                        content: response_content.clone(),
                        tool_calls: None,
                        tool_call_id: None,
                    });

                    state.add_message(ChatMessage {
                        role: Role::Tool,
                        content: result_msg,
                        tool_calls: None,
                        tool_call_id: Some(id.0),
                    });

                    // Track file operations
                    if name == "file_read" {
                        if let Some(path) = arguments.get("path").and_then(|v| v.as_str()) {
                            state.record_file_read(path);
                        }
                    }
                    if name == "file_write" || name == "apply_patch" {
                        if let Some(path) = arguments.get("path").and_then(|v| v.as_str()) {
                            state.record_file_modified(path);
                        }
                    }

                    // Create checkpoint if patch was applied
                    if name == "apply_patch" && result.success {
                        if let Ok(diff) = agent_tools::git::git_diff(&self.working_dir).await {
                            let checkpoint = Checkpoint::new(
                                iteration,
                                diff.diff,
                                format!("patch applied at iteration {iteration}"),
                            );
                            let _ = checkpoint.save(&session_dir).await;
                            self.event_bus
                                .emit(AgentEvent::PatchApplied("patch applied".into()));
                        }
                    }
                }
                AgentAction::NeedMoreContext { queries } => {
                    // Auto-execute search queries
                    for query in &queries {
                        let search_input = agent_tools::types::RgSearchInput {
                            pattern: query.clone(),
                            glob: None,
                            case_sensitive: false,
                            max_results: 20,
                            context_lines: 2,
                        };

                        if let Ok(results) =
                            agent_tools::rg::rg_search(&search_input, &self.working_dir).await
                        {
                            state.add_context(ContextEntry {
                                kind: ContextKind::SearchResult,
                                content: serde_json::to_string_pretty(&results).unwrap_or_default(),
                                source: format!("search:{query}"),
                            });
                        }
                    }

                    state.add_message(ChatMessage {
                        role: Role::Assistant,
                        content: response_content,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
            }
        }

        self.event_bus.emit(AgentEvent::SessionCompleted);

        if final_answer.is_empty() {
            final_answer = "max iterations reached without a final answer".into();
        }

        Ok(final_answer)
    }
}

fn parse_action(response: &str) -> AgentAction {
    // Try to extract JSON from the response
    let content = response.trim();

    // Try direct JSON parse
    if let Ok(action) = serde_json::from_str::<AgentAction>(content) {
        return action;
    }

    // Try to find JSON in code blocks
    if let Some(json_str) = extract_json_block(content) {
        if let Ok(action) = serde_json::from_str::<AgentAction>(&json_str) {
            return action;
        }
    }

    // Try to find any JSON object in the text
    if let Some(start) = content.find('{') {
        if let Some(end) = content.rfind('}') {
            let candidate = &content[start..=end];
            if let Ok(action) = serde_json::from_str::<AgentAction>(candidate) {
                return action;
            }
        }
    }

    // Fallback: treat entire response as an answer
    AgentAction::Answer {
        content: content.to_string(),
    }
}

fn extract_json_block(text: &str) -> Option<String> {
    let markers = ["```json\n", "```\n"];
    for marker in markers {
        if let Some(start) = text.find(marker) {
            let after = &text[start + marker.len()..];
            if let Some(end) = after.find("```") {
                return Some(after[..end].trim().to_string());
            }
        }
    }
    None
}
