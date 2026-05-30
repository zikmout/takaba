# agent-rs

A local-first, modular code agent written in Rust. Designed to run against [Ollama](https://ollama.com/) with models like `qwen2.5-coder:14b` on local hardware (e.g., RTX 4090), with no data leaving your network.

## Overview

agent-rs is a clean-room implementation of an autonomous coding agent. It reads your codebase, searches for relevant code, produces patches, runs quality gates, and manages sessions — all locally, with full control over what the agent can and cannot do.

The project is structured as a Cargo workspace with eight crates, each handling a distinct concern: provider abstraction, tool execution, session management, memory, MCP integration, core agent loop, CLI, and TUI.

## Architecture

```
agent-rs/
├── Cargo.toml                    # workspace root
├── .gitignore
├── Modelfile                     # Ollama custom model definition
├── config.example.toml           # example configuration
├── crates/
│   ├── agent-core/               # agent loop, planner, executor, policy, state, events
│   ├── agent-provider/           # LLM provider trait + Ollama/OpenAI implementations
│   ├── agent-tools/              # local tools (rg, fs, git, shell, patch, project, quality, symbol)
│   ├── agent-mcp/                # Model Context Protocol client via RMCP
│   ├── agent-session/            # session persistence, transcript (JSONL), checkpoints
│   ├── agent-memory/             # short-term memory, repo cache, symbol index, summarizer
│   ├── agent-cli/                # CLI interface (clap)
│   └── agent-tui/                # Terminal UI (ratatui + crossterm)
```

### Crate Dependency Graph

```
agent-cli ──┬── agent-core ──┬── agent-provider
             │                ├── agent-tools
             │                ├── agent-session
             │                └── agent-memory ── agent-provider
             ├── agent-provider
             ├── agent-tools
             ├── agent-mcp
             ├── agent-session
             └── agent-memory

agent-tui ──┬── agent-core
             ├── agent-provider
             ├── agent-tools
             └── agent-session
```

## Crates

### agent-provider

Abstracts LLM communication behind a trait:

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> anyhow::Result<ChatResponse>;
    async fn chat_stream(&self, request: ChatRequest)
        -> anyhow::Result<BoxStream<'static, anyhow::Result<ChatChunk>>>;
    fn metadata(&self) -> ProviderMetadata;
}
```

**Implementations:**

- **`OllamaProvider`** — Full implementation. Sends requests to Ollama's `/api/chat` endpoint with NDJSON streaming. Supports tool calling, configurable timeouts, and retry on network errors.
- **`OpenAICompatibleProvider`** — Stub for any server exposing the OpenAI `/v1/chat/completions` API (vLLM, LM Studio, etc.). Not yet implemented.

**Types:** `ChatRequest`, `ChatMessage`, `ChatResponse`, `ChatChunk`, `ChatOptions`, `ToolDefinition`, `ToolCall`, `FunctionCall`, `FunctionDefinition`, `ProviderMetadata`, `Role`.

### agent-tools

Local tool implementations that the agent can invoke. Each tool has typed input/output structs and is dispatched by name from the executor.

| Tool | Module | Description |
|---|---|---|
| `rg_search` | `rg.rs` | Ripgrep search with JSON output parsing, glob filtering, context lines |
| `file_read` | `fs.rs` | Read files with optional line ranges, binary rejection, size limit (200KB) |
| `file_write` | `fs.rs` | Write files with repo root enforcement, optional directory creation |
| `repo_tree` | `fs.rs` | Directory listing via `ignore` crate (respects `.gitignore`), configurable depth |
| `git_status` | `git.rs` | Branch name + changed files via `git status --short` |
| `git_diff` | `git.rs` | Unstaged + staged diffs |
| `git_commit` | `git.rs` | Stage files and commit (never pushes) |
| `git_branch` | `git.rs` | Create and checkout a new branch |
| `run_command` | `shell.rs` | Execute allowlisted commands with timeout, stdout/stderr capture, truncation |
| `apply_patch` | `patch.rs` | Apply unified diffs via `git apply`, with dry-run, size limit (400 lines), sensitive file blocking |
| `detect_project` | `project.rs` | Detect language (Rust, TypeScript, Go, Python) and infer build/test/lint commands |
| `run_quality_gate` | `quality.rs` | Run format, lint, and test commands based on detected project type |
| `symbol_index` | `symbol.rs` | Extract symbols (functions, structs, enums, traits, classes) via ripgrep patterns |

**ToolRegistry** (`registry.rs`): Unified registry for local and MCP tools. Generates JSON tool definitions for the LLM payload. Tools are registered by name and looked up at dispatch time.

**Security controls:**

- Shell command allowlist: `cargo`, `npm`, `pnpm`, `yarn`, `go`, `python`, `pytest`, `make`, `docker`, `git`, `rg`, `ls`, etc.
- Shell command blocklist patterns: `rm -rf`, `sudo`, `chmod 777`, `dd`, `mkfs`, `nc`, `nmap`, `ssh`, etc.
- Patch sensitive file blocking: `.env`, `.pem`, `.key`, `id_rsa`, `terraform.tfstate`, `credentials`, etc.
- File write repo root enforcement.
- Binary file detection and rejection.

### agent-core

The central crate containing the agent loop and supporting logic.

**`agent_loop.rs`** — Main execution loop:

1. Create or resume a session.
2. Detect project type.
3. Get git status.
4. For each iteration (up to `max_iterations`):
   - Build context pack (system prompt + conversation history).
   - Stream response from the LLM provider.
   - Parse the model's JSON action.
   - Evaluate tool policy.
   - Execute tool call and add result to state.
   - Write transcript entry.
   - If the action is `Answer`, terminate.
5. Create checkpoints when patches are applied.
6. Emit events throughout for CLI/TUI consumption.

**`config.rs`** — Configuration loaded from `.agent/config.toml` with environment variable overrides:

| Config Key | Env Var | Default |
|---|---|---|
| `provider.type` | `AGENT_PROVIDER_TYPE` | `ollama` |
| `provider.base_url` | `AGENT_PROVIDER_URL` | `http://localhost:11434` |
| `provider.model` | `AGENT_MODEL` | `code-agent-14b` |
| `agent.max_iterations` | — | `5` |
| `agent.max_context_tokens` | — | `12000` |
| `agent.max_files_per_iteration` | — | `12` |
| `agent.max_file_size_kb` | — | `200` |
| `git.auto_branch` | — | `true` |
| `git.branch_prefix` | — | `agent/` |

**`policy.rs`** — Tool access control via the `ToolPolicy` trait:

- **Ask mode:** All write tools blocked.
- **Plan mode:** Write tools blocked, commands allowed.
- **Edit mode:** Writes allowed, commits require confirmation.
- **Auto mode:** Full access except push/force operations.
- Sensitive file patterns always blocked for writes.

**`context.rs`** — Builds the system prompt and message array for the LLM, incorporating project metadata, loaded context, conversation history, and tool definitions. Respects the configured token budget.

**`planner.rs`** — Intent classification (query, plan, direct edit, search-then-edit) and search plan generation based on prompt analysis. Extracts keywords and file hints from the user prompt.

**`executor.rs`** — Dispatches tool calls to the appropriate tool implementation, applies policy checks, and emits events.

**`events.rs`** — Event bus using `tokio::sync::broadcast`:

```rust
pub enum AgentEvent {
    SessionStarted(SessionId),
    IterationStarted(usize),
    ToolCallStarted(ToolCallId, String),
    ToolCallFinished(ToolCallId, String),
    ModelToken(String),
    ModelResponseComplete(String),
    PatchProposed(String),
    PatchApplied(String),
    QualityGateStarted,
    QualityGateFinished(bool),
    SessionCompleted,
    Error(String),
}
```

**`state.rs`** — Accumulates context entries (file contents, search results, command outputs, diffs, quality results), tracks files read/modified, and estimates token usage.

**`types.rs`** — Core domain types: `AgentMode` (Ask/Plan/Edit/Auto), `AgentTask`, `AgentAction` (Answer/ToolCall/NeedMoreContext), `ToolCallId`.

**`errors.rs`** — Typed errors via `thiserror`: `Provider`, `Tool`, `PolicyDenied`, `Config`, `Session`, `MaxIterations`, `ContextOverflow`, `Parse`, `Io`, `Json`.

### agent-session

Session persistence and transcript management.

**`session.rs`** — `AgentSession` struct with ID, status, mode, prompt, working directory, timestamps, and iteration count. `SessionStatus`: Active, Completed, Failed, Paused.

**`store.rs`** — `SessionStore` manages session directories under `.agent/sessions/`:

- Directory naming: `{date}_{hh}-{mm}_{slug}/`
- `latest` symlink to most recent session.
- CRUD operations: create, load by ID, list all, update.

**`transcript.rs`** — JSONL transcript with entry types: UserMessage, ModelResponse, ToolCall, ToolResult, Error. Append-only writes, streaming reads.

**`checkpoint.rs`** — Checkpoint snapshots saved per iteration containing git diff and optional quality gate results. Stored as JSON in `{session}/checkpoints/`.

### agent-memory

Memory and caching layer.

**`short_term.rs`** — `ShortTermMemory`: prioritized in-memory store for file contents, search hits, command outputs, diffs, and decisions. Supports deduplication, file invalidation on modification, priority-based eviction, and configurable capacity.

**`repo_cache.rs`** — `RepoCache`: persisted cache (`.agent/cache/repo_cache.json`) tracking repository root, detected stack, and last indexed commit. Supports incremental invalidation based on git HEAD comparison.

**`symbol_index.rs`** — `SymbolIndex`: cached symbol index (`.agent/cache/symbols.json`) with lazy refresh on commit changes. Supports search by name.

**`summarizer.rs`** — Automatic transcript summarization when conversation exceeds a threshold. Calls the LLM to produce a concise summary of key decisions, modified files, errors, and current state.

### agent-mcp

Model Context Protocol (MCP) client layer via RMCP.

**`config.rs`** — `McpServerConfig` with transport configuration (Stdio, SSE, HTTP), enable/disable toggle, and per-server permissions.

**`client.rs`** — `McpClient` handles server lifecycle: connect, discover tools, call tools, shutdown. `connect_all()` connects to all enabled servers from configuration.

**`permissions.rs`** — `McpPermissions` with `allow_read`, `allow_write`, and `require_confirmation_for_write`. Write operations are denied by default; read operations are allowed.

**`types.rs`** — `McpTool`, `McpToolCall`, `McpToolResult`, `McpContent` (Text, Image, Resource).

**Current status:** The MCP client skeleton is in place with RMCP dependency. Actual server connection, tool discovery, and tool execution via RMCP transports are stubbed and marked with TODO.

### agent-cli

Command-line interface built with clap.

```
agent ask <prompt>         # read-only query
agent plan <prompt>        # produce a plan
agent edit <prompt>        # may modify files
agent auto <prompt>        # full autonomous mode
agent test                 # run quality gate
agent status               # git status
agent diff                 # git diff
agent commit -m <message>  # git commit
agent session list         # list sessions
agent session resume <id>  # resume session
agent mcp list             # list tools
agent provider list        # list providers
agent config               # show configuration
```

The CLI:

- Loads configuration from `.agent/config.toml` (with env var overrides).
- Instantiates the Ollama provider.
- Creates the default tool registry.
- Subscribes to the event bus for streaming output.
- Prints model tokens to stderr as they arrive.
- Prints tool call summaries.
- Prints the final answer to stdout.

### agent-tui

Terminal UI built with ratatui and crossterm. Event-driven architecture that consumes `AgentEvent` from the core's event bus.

**Layout:** Three-region split — main panel (70%, switchable between Chat/Diff/Logs/Files), sidebar (30%, files + logs), and status bar.

**Widgets:**

- **Chat** — Streaming token display, user/agent message history, scrollable.
- **Diff** — Colored diff viewer (green for additions, red for removals, cyan for hunks).
- **Files** — List of files read/modified during the session.
- **Logs** — Command execution logs and errors.
- **Status** — Current mode, iteration, active tool, session ID.

**Keyboard shortcuts:**

| Key | Action |
|---|---|
| `Ctrl+C` | Quit |
| `Ctrl+D` | Switch to diff view |
| `Ctrl+T` | Switch to files view |
| `Ctrl+L` | Switch to logs view |
| `Tab` | Cycle panels |
| `Esc` | Return to chat |
| `Enter` | Send prompt |

## Configuration

Copy `config.example.toml` to `.agent/config.toml`:

```toml
[provider]
type = "ollama"
base_url = "http://localhost:11434"
model = "code-agent-14b"

[agent]
max_iterations = 5
max_context_tokens = 12000
max_files_per_iteration = 12
max_file_size_kb = 200

[git]
auto_branch = true
branch_prefix = "agent/"
```

### Environment Variable Overrides

| Variable | Overrides |
|---|---|
| `AGENT_PROVIDER_TYPE` | `provider.type` |
| `AGENT_PROVIDER_URL` | `provider.base_url` |
| `AGENT_MODEL` | `provider.model` |

### MCP Server Configuration

```toml
[[mcp.servers]]
name = "figma"
enabled = true

[mcp.servers.transport]
type = "stdio"
command = "npx"
args = ["-y", "figma-developer-mcp"]

[mcp.servers.permissions]
allow_read = true
allow_write = false
require_confirmation_for_write = true
```

Supported transports: `stdio`, `sse`, `http`.

## Ollama Setup

Create the custom model using the included Modelfile:

```bash
ollama create code-agent-14b -f Modelfile
```

This creates a model based on `qwen2.5-coder:14b` with:

- Low temperature (0.2) for deterministic output.
- JSON action format in the system prompt.
- Tool calling awareness.

Verify the model is available:

```bash
ollama list
```

## Building

```bash
cargo build --workspace
```

Build the CLI in release mode:

```bash
cargo build --release -p agent-cli
```

The binary will be at `target/release/agent-cli`.

## Running

```bash
# Show help
cargo run -p agent-cli -- --help

# Query mode (read-only)
cargo run -p agent-cli -- ask "what does the main function do?"

# Edit mode (can modify files)
cargo run -p agent-cli -- edit "fix the typo in README.md"

# Full auto mode
cargo run -p agent-cli -- auto "add unit tests for the parser module"

# Show current config
cargo run -p agent-cli -- config

# Show git status
cargo run -p agent-cli -- status

# Run quality gate
cargo run -p agent-cli -- test

# List sessions
cargo run -p agent-cli -- session list
```

For the TUI:

```bash
cargo run -p agent-tui
```

## Agent Modes

| Mode | Read | Search | Modify | Commit | Commands |
|---|---|---|---|---|---|
| `ask` | Yes | Yes | No | No | No |
| `plan` | Yes | Yes | No | No | Yes |
| `edit` | Yes | Yes | Yes | Confirm | Yes |
| `auto` | Yes | Yes | Yes | Yes | Yes |

In all modes, push and force operations are blocked.

## Agent Loop

The agent operates in an iterative loop:

1. **Context assembly** — The system prompt is built from project metadata, loaded files, search results, conversation history, and tool definitions. The context is capped at `max_context_tokens`.

2. **LLM call** — The context is sent to the provider via streaming. Tokens are emitted as `ModelToken` events for real-time display.

3. **Action parsing** — The model's response is parsed as a JSON action. The parser tries direct JSON, code blocks, and embedded JSON objects before falling back to treating the entire response as an answer.

4. **Policy check** — Before executing any tool call, the policy is evaluated based on the current mode. Denied actions return an error to the model. Confirmation-required actions are logged.

5. **Tool execution** — The tool is dispatched, executed, and its result is added to the agent state and transcript.

6. **Checkpoint** — If a patch was applied, a checkpoint is saved containing the current git diff and iteration number.

7. **Termination** — The loop ends when the model produces an `Answer` action or `max_iterations` is reached.

## Tool Policy

The `DefaultPolicy` enforces:

- **Mode-based restrictions:** Ask and Plan modes cannot modify files.
- **Push prohibition:** The `run_command` tool blocks any arguments containing `push` or `force`.
- **Sensitive file protection:** Write tools reject paths matching `.env`, `.pem`, `.key`, `id_rsa`, `id_ed25519`, `terraform.tfstate`, `.secret`, `credentials`, `password`.
- **Commit confirmation:** In Edit mode, `git_commit` requires confirmation.

Custom policies can be implemented via the `ToolPolicy` trait.

## Session Management

Sessions are persisted under `.agent/sessions/`:

```
.agent/sessions/
├── 2025-01-15_14-30_fix-parser-bug/
│   ├── session.json          # session metadata
│   ├── transcript.jsonl      # full conversation log
│   └── checkpoints/
│       ├── checkpoint_000.json
│       └── checkpoint_001.json
└── latest -> 2025-01-15_14-30_fix-parser-bug/
```

Each session records:

- Session ID (UUID v4)
- Status (Active, Completed, Failed, Paused)
- Mode, prompt, working directory
- Creation and update timestamps
- Iteration count

The transcript is append-only JSONL with entries for user messages, model responses, tool calls, tool results, and errors.

## Quality Gates

The `run_quality_gate` tool and `agent test` command run checks based on detected project type:

| Language | Format | Lint | Test |
|---|---|---|---|
| Rust | `cargo fmt --check` | `cargo clippy --workspace --all-targets -- -D warnings` | `cargo test --workspace` |
| TypeScript | — | `{pm} lint` | `{pm} test` |
| Go | — | `go vet ./...` | `go test ./...` |
| Python | — | `ruff check .` | `pytest` |

## Dependencies

All dependencies use standard open-source licenses (MIT, Apache-2.0, ISC):

| Category | Crate | Purpose |
|---|---|---|
| Async | `tokio`, `futures`, `async-trait`, `tokio-stream` | Async runtime, streams, trait support |
| Serialization | `serde`, `serde_json`, `toml` | JSON and TOML (de)serialization |
| HTTP | `reqwest` | HTTP client for Ollama API |
| CLI | `clap` | Command-line argument parsing |
| Filesystem | `walkdir`, `ignore` | Directory traversal with gitignore |
| Diff | `similar` | Text diffing |
| Git | `git2` | Git operations (libgit2 bindings) |
| Logging | `tracing`, `tracing-subscriber` | Structured logging |
| TUI | `ratatui`, `crossterm` | Terminal UI framework |
| MCP | `rmcp` | Model Context Protocol client |
| Time | `chrono` | Timestamps |
| IDs | `uuid` | UUID v4 generation |
| Errors | `anyhow`, `thiserror` | Error handling |

## Design Principles

- **Local-first:** No data leaves the local network. The agent communicates only with your Ollama instance.
- **Modular:** Each crate has a single responsibility and clear public API. Swap providers, tools, or policies independently.
- **Safe by default:** Write operations are gated by mode and policy. Push is never allowed. Sensitive files are protected. Commands are allowlisted.
- **Observable:** The event bus provides real-time visibility into agent behavior. Both CLI and TUI consume the same event stream.
- **Resumable:** Sessions persist to disk with full transcripts and checkpoints. Sessions can be listed and resumed.
- **Clean-room:** No code copied from proprietary projects. All dependencies use standard licenses.

## Project Status

This is the initial implementation. The following components are functional:

- Workspace structure and all crates compile cleanly.
- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo check`, and `cargo test` all pass.
- CLI binary runs with all subcommands.
- Ollama provider with streaming.
- Full tool set (rg, fs, git, shell, patch, project, quality, symbol).
- Session persistence and transcript.
- Tool policy enforcement.
- Agent loop with streaming output.
- TUI framework with event-driven architecture.

Components with TODO stubs:

- MCP server connections via RMCP (skeleton in place).
- OpenAI-compatible provider (trait implemented, body is `todo!()`).
- Session resume (metadata loads, state reconstruction pending).
- Transcript summarization (calls provider, integration into loop pending).

## License

All code in this repository is original work. Dependencies are listed above with their respective licenses.
