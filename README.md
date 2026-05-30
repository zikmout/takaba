# agent-rs

A local-first, modular code agent written in Rust. Designed to run against [Ollama](https://ollama.com/) with models like `qwen2.5-coder:14b` on local hardware (e.g., RTX 4090), with no data leaving your network.

---

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Crate Reference](#crate-reference)
  - [agent-provider](#agent-provider)
  - [agent-tools](#agent-tools)
  - [agent-core](#agent-core)
  - [agent-session](#agent-session)
  - [agent-memory](#agent-memory)
  - [agent-mcp](#agent-mcp)
  - [agent-cli](#agent-cli)
  - [agent-tui](#agent-tui)
- [Configuration](#configuration)
- [Ollama Setup](#ollama-setup)
- [Agent Modes](#agent-modes)
- [Agent Loop Internals](#agent-loop-internals)
- [Action Protocol](#action-protocol)
- [Tool Policy and Security](#tool-policy-and-security)
- [Session Management](#session-management)
- [Quality Gates](#quality-gates)
- [MCP Integration](#mcp-integration)
- [Data Flow](#data-flow)
- [Extending agent-rs](#extending-agent-rs)
- [File-by-File Module Reference](#file-by-file-module-reference)
- [Dependencies](#dependencies)
- [Troubleshooting](#troubleshooting)
- [Design Principles](#design-principles)
- [Project Status](#project-status)
- [Roadmap](#roadmap)
- [Clean-Room Policy](#clean-room-policy)
- [License](#license)

---

## Overview

agent-rs is a clean-room implementation of an autonomous coding agent. It reads your codebase, searches for relevant code, produces patches, runs quality gates, and manages sessions -- all locally, with full control over what the agent can and cannot do.

Key characteristics:

- **Fully local execution.** The agent communicates only with your Ollama instance on your LAN. No API keys, no cloud, no telemetry.
- **Optimized for local LLMs.** Tuned context budgets, compact prompts, and streaming designed for models running on consumer GPUs (tested with qwen2.5-coder:14b on RTX 4090).
- **Git-aware.** Every modification goes through git. The agent never pushes, never force-overwrites, and creates checkpoints after patches.
- **Policy-controlled.** A pluggable policy system gates every tool call based on the operating mode, blocking writes in read-only modes, protecting sensitive files, and enforcing command allowlists.
- **Observable.** A broadcast event bus lets any frontend (CLI, TUI, or custom) subscribe to real-time agent activity.

The project is structured as a Cargo workspace with eight crates, each handling a distinct concern.

---

## Prerequisites

| Requirement | Minimum Version | Notes |
|---|---|---|
| Rust toolchain | 1.75+ | Install via [rustup](https://rustup.rs/) |
| Ollama | 0.1.0+ | Install from [ollama.com](https://ollama.com/) |
| ripgrep (`rg`) | 13.0+ | Required by `rg_search` and `symbol_index` tools |
| git | 2.30+ | Required by all git tools and `apply_patch` |
| libgit2 system deps | — | Installed automatically by the `git2` crate build script |
| OpenSSL dev headers | — | Required by `reqwest` TLS on Linux (`libssl-dev` / `openssl-devel`) |

Optional:

| Requirement | Notes |
|---|---|
| Node.js / npx | Required only if using MCP servers that launch via npx |
| Docker | Required only if the agent needs to run docker commands |

---

## Installation

### From source

```bash
git clone <your-repo-url> agent-rs
cd agent-rs
cargo build --release --workspace
```

The CLI binary will be at `target/release/agent-cli`.
The TUI binary will be at `target/release/agent-tui`.

### Install the CLI globally

```bash
cargo install --path crates/agent-cli
```

This places `agent-cli` in `~/.cargo/bin/`.

### Verify the build

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace
cargo test --workspace
```

All four commands should pass cleanly.

---

## Quick Start

```bash
# 1. Create the Ollama model
ollama create code-agent-14b -f Modelfile

# 2. (Optional) Create a project config
mkdir -p .agent
cp config.example.toml .agent/config.toml

# 3. Run in query mode
cargo run -p agent-cli -- ask "explain the project structure"

# 4. Run in edit mode
cargo run -p agent-cli -- edit "add a doc comment to the main function"

# 5. Check quality
cargo run -p agent-cli -- test
```

---

## Architecture

### Workspace Layout

```
agent-rs/
├── Cargo.toml                    # workspace root, shared dependencies
├── Cargo.lock                    # locked dependency versions
├── .gitignore                    # ignores target/, .agent/, *.log
├── Modelfile                     # Ollama custom model definition
├── config.example.toml           # example configuration file
├── README.md
├── crates/
│   ├── agent-core/               # agent loop, planner, executor, policy, state, events, config
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

The dependency graph is intentionally acyclic. `agent-provider` and `agent-tools` have no cross-dependencies, making them independently testable. `agent-core` aggregates them into the agent loop. The CLI and TUI are thin frontends that instantiate the core and subscribe to events.

---

## Crate Reference

### agent-provider

Abstracts LLM communication behind a trait. This is the only crate that makes HTTP calls to an inference server.

#### Provider Trait

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a chat request and receive a complete response.
    async fn chat(&self, request: ChatRequest) -> anyhow::Result<ChatResponse>;

    /// Send a chat request and receive a streaming response.
    async fn chat_stream(&self, request: ChatRequest)
        -> anyhow::Result<BoxStream<'static, anyhow::Result<ChatChunk>>>;

    /// Return metadata about this provider (name, model, base URL).
    fn metadata(&self) -> ProviderMetadata;
}
```

#### Implementations

| Provider | Module | Status | Description |
|---|---|---|---|
| `OllamaProvider` | `ollama.rs` | Complete | Ollama `/api/chat` with NDJSON streaming, tool calling, configurable timeout (default 300s) |
| `OpenAICompatibleProvider` | `openai_compat.rs` | Stub | For vLLM, LM Studio, or any OpenAI-compatible `/v1/chat/completions` endpoint |

#### Types

| Type | Description |
|---|---|
| `ChatRequest` | Model name, messages array, optional tools and options |
| `ChatMessage` | Role (System/User/Assistant/Tool), content, optional tool calls |
| `ChatResponse` | Complete response with message, done flag, timing stats |
| `ChatChunk` | Single streaming chunk with partial message and done flag |
| `ChatOptions` | Temperature, top_p, num_predict, stop sequences |
| `ToolDefinition` | JSON Schema tool definition for the LLM payload |
| `ToolCall` | Tool invocation with ID, type, and function call |
| `FunctionCall` | Function name and JSON-encoded arguments string |
| `FunctionDefinition` | Function name, description, and parameter JSON Schema |
| `ProviderMetadata` | Provider name, model name, base URL |
| `Role` | Enum: System, User, Assistant, Tool |

#### Streaming

`streaming.rs` provides `parse_ndjson_stream()` which converts a `reqwest::Response` into a `BoxStream<ChatChunk>`. It handles:

- Incremental buffer accumulation across TCP chunk boundaries.
- Line-by-line NDJSON parsing.
- Graceful handling of malformed lines (logged and skipped).
- Flushing remaining buffer data after stream completion.

---

### agent-tools

Local tool implementations that the agent can invoke. Each tool has strongly typed input/output structs defined in `types.rs` and is dispatched by name from the executor.

#### Tool Inventory

| Tool | Module | Input Type | Output Type | Description |
|---|---|---|---|---|
| `rg_search` | `rg.rs` | `RgSearchInput` | `RgSearchOutput` | Ripgrep search with `--json` output, glob filtering, case sensitivity, context lines, result limit |
| `file_read` | `fs.rs` | `FileReadInput` | `FileReadOutput` | Read file with optional line range, binary rejection, 200KB size limit |
| `file_write` | `fs.rs` | `FileWriteInput` | `FileWriteOutput` | Write file with repo root enforcement, optional parent directory creation |
| `repo_tree` | `fs.rs` | `RepoTreeInput` | `RepoTreeOutput` | Directory listing via `ignore` crate (respects `.gitignore`), configurable max depth |
| `git_status` | `git.rs` | — | `GitStatusOutput` | Current branch + changed files via `git status --short` |
| `git_diff` | `git.rs` | — | `GitDiffOutput` | Both unstaged (`git diff`) and staged (`git diff --staged`) diffs |
| `git_commit` | `git.rs` | `GitCommitInput` | `GitCommitOutput` | Stage specified files and commit. Never pushes. |
| `git_branch` | `git.rs` | name (string) | branch name | `git checkout -b <name>` |
| `run_command` | `shell.rs` | `ShellCommandInput` | `ShellCommandOutput` | Execute allowlisted command with timeout, stdout/stderr capture, 64KB output truncation |
| `apply_patch` | `patch.rs` | `PatchInput` | `PatchOutput` | Apply unified diff via `git apply --verbose`, dry-run support, 400-line limit, sensitive file blocking |
| `detect_project` | `project.rs` | — | `ProjectMetadata` | Detect language and infer package manager, test/lint/typecheck/build commands |
| `run_quality_gate` | `quality.rs` | — | `QualityGateResult` | Run format, lint, test commands based on detected project type |
| `symbol_index` | `symbol.rs` | optional glob | `Vec<Symbol>` | Extract symbols (functions, structs, enums, traits, classes, interfaces) via ripgrep regex patterns |

#### ToolRegistry

`registry.rs` provides `ToolRegistry`, a unified registry for local and MCP tools:

```rust
pub struct ToolRegistry {
    tools: HashMap<String, RegisteredTool>,
}

pub enum RegisteredTool {
    Local(LocalToolDefinition),
    Mcp(McpToolEntry),
}
```

- `register_local()` / `register_mcp()` — Register tools.
- `list_tools()` — List all registered tools.
- `get_tool()` — Lookup by name.
- `tool_definitions()` — Generate `Vec<serde_json::Value>` for the LLM payload.
- `has_tool()` — Check existence.

`create_default_registry()` returns a registry pre-populated with all 13 built-in tools and their JSON Schema parameter definitions.

#### Security Controls

**Shell command allowlist** (enforced in `shell.rs`):
```
cargo, npm, pnpm, yarn, go, python, python3, pytest, make, docker,
git, rg, ls, cat, head, tail, wc, sort, uniq, grep, find, tree
```

**Shell command blocklist patterns:**
```
rm -rf, curl|sh, wget|sh, sudo, chmod 777, dd, mkfs, nc, nmap,
ssh, scp, > /dev/, :(){ :
```

**Patch sensitive file patterns** (enforced in `patch.rs`):
```
.env, .pem, .key, id_rsa, id_ed25519, terraform.tfstate, .secret, credentials
```

**File I/O protections:**
- `file_read` rejects binary files (null byte detection in first 8KB) and files over 200KB.
- `file_write` verifies the target path is within the repository root via canonicalization.
- `repo_tree` respects `.gitignore` rules via the `ignore` crate.

#### Project Detection

`project.rs` detects the project type by checking for configuration files:

| File | Detected Language | Package Manager | Test Command | Lint Command |
|---|---|---|---|---|
| `Cargo.toml` | Rust | cargo | `cargo test` | `cargo clippy -- -D warnings` |
| `package.json` + `pnpm-lock.yaml` | TypeScript | pnpm | `pnpm test` | `pnpm lint` |
| `package.json` + `yarn.lock` | TypeScript | yarn | `yarn test` | `yarn lint` |
| `package.json` | TypeScript | npm | `npm test` | `npm lint` |
| `go.mod` | Go | go | `go test ./...` | `golangci-lint run` |
| `pyproject.toml` / `setup.py` | Python | pip | `pytest` | `ruff check .` |

---

### agent-core

The central crate containing the agent loop, configuration, policy engine, planner, executor, event system, and state management.

#### Module Breakdown

**`agent_loop.rs`** — The main execution loop (`AgentLoop` struct):

1. Create a new session (or resume an existing one).
2. Run `detect_project` to identify the codebase language/stack.
3. Run `git_status` to capture the initial repository state.
4. Enter the iteration loop (capped at `max_iterations`):
   a. Build the context pack (system prompt + messages + tool definitions).
   b. Send to the LLM via `chat_stream()`.
   c. Accumulate streamed tokens, emitting `ModelToken` events.
   d. Parse the complete response as a JSON `AgentAction`.
   e. If `Answer` — store the final answer and break.
   f. If `ToolCall` — check policy, execute, record transcript, add result to state.
   g. If `NeedMoreContext` — auto-execute the requested search queries and add results to state.
   h. If a patch was applied, save a checkpoint.
5. Emit `SessionCompleted` and return the final answer.

**`config.rs`** — `AgentConfig` loaded from `.agent/config.toml`:

| Section | Field | Type | Default | Description |
|---|---|---|---|---|
| `provider` | `type` | String | `"ollama"` | Provider backend |
| `provider` | `base_url` | String | `"http://localhost:11434"` | Inference server URL |
| `provider` | `model` | String | `"code-agent-14b"` | Model name |
| `agent` | `max_iterations` | usize | `5` | Max tool-call iterations per session |
| `agent` | `max_context_tokens` | usize | `12000` | Token budget for context window |
| `agent` | `max_files_per_iteration` | usize | `12` | Max files to include in context |
| `agent` | `max_file_size_kb` | usize | `200` | Max single file size |
| `git` | `auto_branch` | bool | `true` | Auto-create agent branches |
| `git` | `branch_prefix` | String | `"agent/"` | Branch name prefix |

Environment variable overrides: `AGENT_PROVIDER_TYPE`, `AGENT_PROVIDER_URL`, `AGENT_MODEL`.

**`policy.rs`** — The `ToolPolicy` trait and `DefaultPolicy`:

```rust
#[async_trait]
pub trait ToolPolicy: Send + Sync {
    async fn check(&self, tool_name: &str, arguments: &Value, mode: AgentMode) -> PolicyResult;
}

pub enum PolicyResult {
    Allow,
    Deny(String),
    RequireConfirmation(String),
}
```

**`context.rs`** — `build_context_pack()` assembles the message array:

1. System prompt with mode instructions, project info, context summary, tool count, and JSON action format.
2. User task prompt.
3. Conversation history (messages from previous iterations), capped by `max_context_tokens`.

**`planner.rs`** — Intent classification and search plan generation:

- `classify_intent()` maps mode + prompt to an `Intent` enum (Query, Plan, DirectEdit, SearchThenEdit).
- `generate_search_plan()` produces a sequence of `SearchStep` values (DetectProject, GitStatus, Search, ReadFile).
- Keyword extraction filters stop words and selects up to 5 meaningful terms.
- File hint extraction recognizes common source file extensions in the prompt.

**`executor.rs`** — `execute_tool_call()` dispatches to the correct tool:

1. Check policy via `ToolPolicy::check()`.
2. Emit `ToolCallStarted` event.
3. Deserialize arguments into the typed input struct.
4. Call the tool function.
5. Emit `ToolCallFinished` event.
6. Return `ToolResult` with success/failure and output.

**`events.rs`** — `EventBus` wraps `tokio::sync::broadcast`:

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

Default channel capacity: 256 events. Events are fire-and-forget (no error if no subscribers).

**`state.rs`** — `AgentState` accumulates:

- `messages: Vec<ChatMessage>` — Conversation history.
- `context_entries: Vec<ContextEntry>` — Loaded context (files, search results, diffs, etc.).
- `project: Option<ProjectMetadata>` — Detected project info.
- `iteration: usize` — Current iteration number.
- `files_read: Vec<String>` — Paths read during the session.
- `files_modified: Vec<String>` — Paths modified during the session.
- `last_diff: Option<String>` — Most recent git diff.
- `estimated_tokens()` — Rough token count (content length / 4).

Context entries are deduplicated by source and kind.

**`types.rs`** — Core domain types:

| Type | Description |
|---|---|
| `AgentMode` | Enum: Ask, Plan, Edit, Auto |
| `AgentTask` | Prompt + mode + working directory |
| `AgentAction` | Enum: Answer, ToolCall, NeedMoreContext |
| `ToolCallId` | UUID v4 wrapper for correlating tool calls and results |

**`errors.rs`** — `AgentError` enum (via `thiserror`):

| Variant | Description |
|---|---|
| `Provider(String)` | LLM provider communication error |
| `Tool { tool, message }` | Tool execution failure |
| `PolicyDenied(String)` | Policy rejected a tool call |
| `Config(String)` | Configuration loading error |
| `Session(String)` | Session management error |
| `MaxIterations(usize)` | Iteration limit reached |
| `ContextOverflow { used, max }` | Token budget exceeded |
| `Parse(String)` | Response parsing failure |
| `Io` | I/O error (transparent from `std::io::Error`) |
| `Json` | JSON error (transparent from `serde_json::Error`) |
| `Other` | Catch-all (transparent from `anyhow::Error`) |

---

### agent-session

Session persistence and transcript management. All data is stored as flat files in `.agent/sessions/`.

**`session.rs`** — `AgentSession` struct:

| Field | Type | Description |
|---|---|---|
| `id` | `SessionId` (UUID v4) | Unique session identifier |
| `status` | `SessionStatus` | Active, Completed, Failed, Paused |
| `mode` | `String` | Operating mode at session creation |
| `prompt` | `String` | Original user prompt |
| `working_dir` | `PathBuf` | Repository root |
| `created_at` | `DateTime<Utc>` | Session creation timestamp |
| `updated_at` | `DateTime<Utc>` | Last update timestamp |
| `iteration_count` | `usize` | Number of iterations completed |

**`store.rs`** — `SessionStore`:

- Sessions stored in `.agent/sessions/{date}_{hh}-{mm}_{slug}/`.
- `latest` symlink updated on each new session (Unix only).
- Operations: `create()`, `load(id)`, `list()`, `update()`.
- Slug generated from first 30 characters of the prompt.

**`transcript.rs`** — Append-only JSONL transcript:

```rust
pub enum TranscriptKind {
    UserMessage { content },
    ModelResponse { content },
    ToolCall { id, name, arguments },
    ToolResult { id, name, success, output },
    Error { message },
}
```

Each entry includes a UTC timestamp. The transcript file is `transcript.jsonl` inside the session directory.

**`checkpoint.rs`** — Checkpoint snapshots:

- Saved as `checkpoints/checkpoint_{NNN}.json` inside the session directory.
- Contains: timestamp, iteration number, full git diff, optional quality gate result, description.
- `Checkpoint::list()` loads all checkpoints sorted by iteration.

---

### agent-memory

Memory and caching layer for context management across iterations and sessions.

**`short_term.rs`** — `ShortTermMemory`:

- In-memory store with configurable capacity (default 100 entries).
- Entry types: FileContent, SearchHit, CommandOutput, Diff, Decision.
- Each entry has a priority (0-255) and timestamp.
- Deduplication by source on insert.
- File invalidation: when a file is modified, its cached content is evicted.
- Eviction: when capacity is exceeded, lowest-priority oldest entries are removed first.
- `get_recent(limit)` returns the most recent N entries.
- `get_by_kind(kind)` filters entries by type.
- `estimated_tokens()` sums content lengths / 4.

**`repo_cache.rs`** — `RepoCache`:

- Persisted to `.agent/cache/repo_cache.json`.
- Tracks: repository root, detected stack, last indexed commit hash.
- `needs_refresh()` compares current `git rev-parse HEAD` against `last_indexed_commit`.
- `update_commit()` updates the stored commit hash to current HEAD.

**`symbol_index.rs`** — `SymbolIndex`:

- Persisted to `.agent/cache/symbols.json`.
- Stores: list of `SymbolEntry` (name, kind, file, line), indexed commit hash.
- `needs_refresh()` checks commit hash like `RepoCache`.
- `search(query)` performs case-insensitive substring matching on symbol names.

**`summarizer.rs`** — Transcript summarization:

- `needs_summarization(messages, max_entries)` checks if the message count exceeds a threshold.
- `summarize_transcript()` sends the full transcript to the LLM with a summarization prompt.
- The summary focuses on: key decisions, files modified, errors encountered, and current state.
- Output is capped to 500 words.

---

### agent-mcp

Model Context Protocol (MCP) client layer. Allows the agent to connect to external tool servers (Figma, databases, Gitea, etc.) via the standardized MCP protocol.

**`config.rs`** — Server configuration:

```toml
[[mcp.servers]]
name = "figma"
enabled = true

[mcp.servers.transport]
type = "stdio"          # or "sse" or "http"
command = "npx"
args = ["-y", "figma-developer-mcp"]
env = {}

[mcp.servers.permissions]
allow_read = true
allow_write = false
require_confirmation_for_write = true
```

**`client.rs`** — `McpClient`:

| Method | Description |
|---|---|
| `new(config)` | Create a client from server config |
| `connect()` | Establish connection (stdio/SSE/HTTP) |
| `discover_tools()` | Retrieve available tools from the server |
| `call_tool(call)` | Execute a tool call with permission checks |
| `shutdown()` | Gracefully close the connection |
| `is_connected()` | Check connection status |

`connect_all(configs)` connects to all enabled servers and returns a `HashMap<String, McpClient>`.

Write operation detection uses keyword matching (`write`, `create`, `update`, `delete`, `modify`, `set`, `push`, `merge`).

**`permissions.rs`** — `McpPermissions`:

| Field | Default | Description |
|---|---|---|
| `allow_read` | `true` | Whether read operations are permitted |
| `allow_write` | `false` | Whether write operations are permitted |
| `require_confirmation_for_write` | `true` | Whether writes need user confirmation |

**`types.rs`** — `McpTool`, `McpToolCall`, `McpToolResult`, `McpContent` (Text, Image, Resource).

**Current status:** The client skeleton and configuration are in place. Actual RMCP transport connections are stubbed with TODO markers.

---

### agent-cli

Command-line interface built with [clap](https://docs.rs/clap/) v4 with derive macros.

#### Commands

| Command | Description |
|---|---|
| `agent-cli ask <prompt>` | Read-only query mode. The agent can search and read but not modify files. |
| `agent-cli plan <prompt>` | Planning mode. The agent analyzes the codebase and produces a structured plan. |
| `agent-cli edit <prompt>` | Edit mode. The agent may modify files. Commits require confirmation. |
| `agent-cli auto <prompt>` | Full autonomous mode. The agent can read, write, commit, and run commands. |
| `agent-cli test` | Run the quality gate for the detected project type. |
| `agent-cli status` | Display current git branch and changed files. |
| `agent-cli diff` | Display staged and unstaged git diffs. |
| `agent-cli commit -m <message>` | Stage all changed files and create a commit. |
| `agent-cli session list` | List all saved sessions with ID, status, date, and prompt. |
| `agent-cli session resume <id>` | Resume a previously saved session (partial implementation). |
| `agent-cli mcp list` | List all registered tools (local + MCP). |
| `agent-cli provider list` | List available LLM providers and their configuration. |
| `agent-cli config` | Print the current configuration as TOML. |

#### Runtime Behavior

1. Loads `AgentConfig` from `.agent/config.toml` (falls back to defaults).
2. Initializes `tracing_subscriber` with `RUST_LOG` env filter support.
3. For agent commands (ask/plan/edit/auto):
   - Instantiates `OllamaProvider`.
   - Creates the default `ToolRegistry`.
   - Creates a shared `EventBus`.
   - Spawns an async task that subscribes to events and prints them to stderr.
   - Runs `AgentLoop::run()`.
   - Prints the final answer to stdout.
4. For utility commands (test/status/diff/commit): executes directly without the agent loop.

#### Output Channels

- **stderr** — Real-time agent events: session lifecycle, iteration numbers, tool calls, streaming tokens, errors.
- **stdout** — Final answer only (suitable for piping).

---

### agent-tui

Terminal UI built with [ratatui](https://docs.rs/ratatui/) 0.29 and [crossterm](https://docs.rs/crossterm/) 0.28. Fully event-driven: the TUI consumes `AgentEvent` from the core's event bus and never contains business logic.

#### Layout

```
┌─────────────────────────────────┬──────────────┐
│                                 │   Files      │
│         Main Panel              │              │
│   (Chat / Diff / Logs / Files)  ├──────────────┤
│                                 │   Logs       │
│                                 │              │
├─────────────────────────────────┴──────────────┤
│  mode: edit | iter: 2 | tool: rg_search        │
└─────────────────────────────────────────────────┘
```

- Main panel (70%): switchable between Chat, Diff, Logs, and Files views.
- Sidebar (30%): always-visible files list (top) and logs (bottom).
- Status bar (1 line): current mode, iteration, active tool, session ID.

#### Widgets

| Widget | Module | Description |
|---|---|---|
| Chat | `widgets/chat.rs` | Streaming token display with user/agent message history, scrollable |
| Diff | `widgets/diff.rs` | Colored unified diff viewer (green additions, red removals, cyan hunks) |
| Files | `widgets/files.rs` | List of files read/modified during the session |
| Logs | `widgets/logs.rs` | Chronological command execution logs and error messages |
| Status | `widgets/status.rs` | Mode, iteration, active tool, truncated session ID |

#### Keyboard Shortcuts

| Key | Action |
|---|---|
| `Ctrl+C` | Quit the TUI |
| `Ctrl+D` | Switch main panel to diff view |
| `Ctrl+T` | Switch main panel to files view |
| `Ctrl+L` | Switch main panel to logs view |
| `Tab` | Cycle through panels (Chat -> Diff -> Logs -> Files) |
| `Esc` | Return to chat view |
| `Enter` | Send the current input as a prompt |
| `Backspace` | Delete last character in input |

#### Event System

`TuiEventHandler` merges three event sources into a single `mpsc` channel:

1. **Crossterm events** — Keyboard input and terminal resize, polled in a dedicated thread every 100ms.
2. **Agent events** — `AgentEvent` from the broadcast bus, received in an async task.
3. **Tick events** — 250ms periodic ticks for UI refresh.

---

## Configuration

### Configuration File

The agent looks for `.agent/config.toml` in the working directory. If not found, defaults are used.

```toml
[provider]
type = "ollama"                   # Provider backend: "ollama" or "openai_compat"
base_url = "http://localhost:11434"  # Inference server URL
model = "code-agent-14b"          # Model name

[agent]
max_iterations = 5                # Max tool-call iterations per session
max_context_tokens = 12000        # Token budget for the context window
max_files_per_iteration = 12      # Max files to include in a single context
max_file_size_kb = 200            # Max single file size (files larger are rejected)

[git]
auto_branch = true                # Automatically create agent branches before edits
branch_prefix = "agent/"          # Prefix for auto-created branch names
```

### Environment Variable Overrides

Environment variables take precedence over the config file:

| Variable | Overrides | Example |
|---|---|---|
| `AGENT_PROVIDER_TYPE` | `provider.type` | `AGENT_PROVIDER_TYPE=openai_compat` |
| `AGENT_PROVIDER_URL` | `provider.base_url` | `AGENT_PROVIDER_URL=http://192.168.1.100:11434` |
| `AGENT_MODEL` | `provider.model` | `AGENT_MODEL=qwen2.5-coder:32b` |

### Logging

Set `RUST_LOG` to control log verbosity:

```bash
RUST_LOG=debug cargo run -p agent-cli -- ask "hello"
RUST_LOG=agent_core=trace,agent_tools=debug cargo run -p agent-cli -- edit "fix bug"
```

---

## Ollama Setup

### Create the Custom Model

```bash
ollama create code-agent-14b -f Modelfile
```

The `Modelfile` configures:

| Parameter | Value | Purpose |
|---|---|---|
| Base model | `qwen2.5-coder:14b` | Code-specialized 14B parameter model |
| `temperature` | `0.2` | Low randomness for deterministic, focused output |
| `top_p` | `0.9` | Nucleus sampling threshold |
| `num_predict` | `4096` | Maximum tokens to generate per response |
| `stop` | `<\|endoftext\|>`, `<\|end\|>` | Stop sequences |

The system prompt instructs the model to respond with JSON actions and lists all available tools.

### Verify

```bash
ollama list                     # Should show code-agent-14b
ollama run code-agent-14b       # Interactive test
```

### Network Setup

If Ollama runs on a different machine (e.g., GPU server on your LAN):

```bash
# On the Ollama server, bind to all interfaces:
OLLAMA_HOST=0.0.0.0:11434 ollama serve

# On the agent machine, point to the server:
AGENT_PROVIDER_URL=http://192.168.1.100:11434 cargo run -p agent-cli -- ask "hello"
```

---

## Agent Modes

| Mode | Can Read | Can Search | Can Modify | Can Commit | Can Run Commands | Push Allowed |
|---|---|---|---|---|---|---|
| `ask` | Yes | Yes | No | No | No | No |
| `plan` | Yes | Yes | No | No | Yes | No |
| `edit` | Yes | Yes | Yes | Requires confirmation | Yes | No |
| `auto` | Yes | Yes | Yes | Yes | Yes | No |

Push and force operations are blocked in all modes, unconditionally.

---

## Agent Loop Internals

### Iteration Lifecycle

```
┌─ Iteration N ──────────────────────────────────────────────────┐
│                                                                │
│  1. build_context_pack(state, prompt, mode, config, tools)     │
│     └─> [system_prompt, user_msg, ...history]                  │
│                                                                │
│  2. provider.chat_stream(request)                              │
│     └─> tokens emitted as ModelToken events                    │
│                                                                │
│  3. parse_action(response_text)                                │
│     ├─ Try: direct JSON parse                                  │
│     ├─ Try: extract from ```json code block                    │
│     ├─ Try: find first { ... last } in text                    │
│     └─ Fallback: treat entire response as Answer               │
│                                                                │
│  4. match action:                                              │
│     ├─ Answer       → store, break loop                        │
│     ├─ ToolCall     → policy check → execute → record          │
│     └─ NeedMoreCtx  → auto-search queries → add to state      │
│                                                                │
│  5. If patch applied → save checkpoint                         │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

### Context Window Management

The system prompt is built dynamically based on:

- Operating mode (determines what instructions the model receives).
- Detected project metadata (language, package manager).
- Number of loaded files and search results.
- List of files already read.
- Number of available tools.
- JSON action format specification.

Conversation history is appended until the token budget (`max_context_tokens`) is exhausted. Messages are added oldest-first; once the budget is hit, remaining messages are dropped.

---

## Action Protocol

The model communicates with the agent via JSON objects. Three action types are supported:

### Answer

```json
{"type": "answer", "content": "The main function initializes the logger and starts the server."}
```

Terminates the current session and returns the content to the user.

### ToolCall

```json
{
  "type": "tool_call",
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "rg_search",
  "arguments": {"pattern": "fn main", "glob": "*.rs"}
}
```

Invokes the named tool with the given arguments. The result is added to the conversation as a Tool message and the loop continues.

### NeedMoreContext

```json
{
  "type": "need_more_context",
  "queries": ["authentication middleware", "JWT token validation"]
}
```

The agent auto-executes ripgrep searches for each query and adds the results to the state, then continues the loop.

---

## Tool Policy and Security

### DefaultPolicy Rules

| Rule | Mode | Tool(s) | Effect |
|---|---|---|---|
| Block writes in read-only modes | Ask | file_write, apply_patch, git_commit, git_branch, run_command | Deny |
| Block writes in planning mode | Plan | file_write, apply_patch, git_commit, git_branch | Deny |
| Block push/force everywhere | All | run_command (if args contain "push" or "force") | Deny |
| Protect sensitive files | All | file_write, apply_patch | Deny if path matches pattern |
| Require commit confirmation | Edit | git_commit | RequireConfirmation |

### Sensitive File Patterns

The following patterns are blocked from write operations: `.env`, `.pem`, `.key`, `id_rsa`, `id_ed25519`, `terraform.tfstate`, `.secret`, `credentials`, `password`.

### Custom Policies

Implement the `ToolPolicy` trait to create custom access control:

```rust
#[async_trait]
impl ToolPolicy for MyPolicy {
    async fn check(&self, tool_name: &str, arguments: &Value, mode: AgentMode) -> PolicyResult {
        // Your logic here
        PolicyResult::Allow
    }
}
```

---

## Session Management

### Directory Structure

```
.agent/
├── config.toml
├── cache/
│   ├── repo_cache.json
│   └── symbols.json
└── sessions/
    ├── 2025-01-15_14-30_fix-parser-bug/
    │   ├── session.json
    │   ├── transcript.jsonl
    │   └── checkpoints/
    │       ├── checkpoint_000.json
    │       └── checkpoint_001.json
    ├── 2025-01-16_09-15_add-auth-middleware/
    │   ├── session.json
    │   └── transcript.jsonl
    └── latest -> 2025-01-16_09-15_add-auth-middleware/
```

### Session Metadata (`session.json`)

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "mode": "edit",
  "prompt": "fix the parser bug in src/parser.rs",
  "working_dir": "/home/user/project",
  "created_at": "2025-01-15T14:30:00Z",
  "updated_at": "2025-01-15T14:32:15Z",
  "iteration_count": 3
}
```

### Transcript Format (`transcript.jsonl`)

Each line is a JSON object:

```jsonl
{"timestamp":"2025-01-15T14:30:01Z","kind":{"type":"user_message","content":"fix the parser bug"}}
{"timestamp":"2025-01-15T14:30:05Z","kind":{"type":"tool_call","id":"abc","name":"rg_search","arguments":{"pattern":"parse"}}}
{"timestamp":"2025-01-15T14:30:06Z","kind":{"type":"tool_result","id":"abc","name":"rg_search","success":true,"output":"..."}}
{"timestamp":"2025-01-15T14:30:10Z","kind":{"type":"model_response","content":"{\"type\":\"answer\",\"content\":\"Fixed.\"}"}}
```

### Checkpoint Format

```json
{
  "timestamp": "2025-01-15T14:31:00Z",
  "iteration": 2,
  "git_diff": "diff --git a/src/parser.rs ...",
  "quality_result": {"passed": true, "summary": "all checks passed"},
  "description": "patch applied at iteration 2"
}
```

---

## Quality Gates

### Supported Languages

| Language | Format Check | Lint | Test |
|---|---|---|---|
| Rust | `cargo fmt --check` | `cargo clippy --workspace --all-targets -- -D warnings` | `cargo test --workspace` |
| TypeScript/JS | — | `{pm} lint` | `{pm} test` |
| Go | — | `go vet ./...` | `go test ./...` |
| Python | — | `ruff check .` | `pytest` |

Where `{pm}` is the detected package manager (pnpm, yarn, or npm).

### CLI Usage

```bash
cargo run -p agent-cli -- test
```

Output:

```
Running quality gate for rust project...
[PASS] fmt
[PASS] clippy
[PASS] test

All checks passed.
```

Exit code is 0 on success, 1 on failure.

---

## MCP Integration

### Overview

MCP (Model Context Protocol) allows the agent to use external tools provided by third-party servers. The agent connects to MCP servers via stdio, SSE, or HTTP transports using the [RMCP](https://crates.io/crates/rmcp) crate.

### Configuration

Add servers to `.agent/config.toml`:

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

[[mcp.servers]]
name = "database"
enabled = true

[mcp.servers.transport]
type = "sse"
url = "http://localhost:8080/mcp"

[mcp.servers.permissions]
allow_read = true
allow_write = false
```

### Tool Naming

MCP tools are registered in the `ToolRegistry` with the naming convention `mcp.{server}.{tool}`. For example, a tool called `inspect_selection` from the `figma` server would be registered as `mcp.figma.inspect_selection`.

### Permission Model

- Read operations are allowed by default.
- Write operations are denied by default.
- Write confirmation can be required or auto-approved per server.
- The agent classifies operations as write if the tool name contains: `write`, `create`, `update`, `delete`, `modify`, `set`, `push`, `merge`.

---

## Data Flow

```
User Prompt
    │
    ▼
AgentTask { prompt, mode, working_dir }
    │
    ▼
AgentLoop::run()
    │
    ├─► detect_project() ──► ProjectMetadata ──► AgentState.project
    ├─► git_status()     ──► GitStatusOutput  ──► AgentState.context_entries
    │
    │   ┌─── Iteration Loop ───────────────────────────────────────┐
    │   │                                                          │
    │   │  build_context_pack() ──► Vec<ChatMessage>               │
    │   │      │                                                   │
    │   │      ▼                                                   │
    │   │  provider.chat_stream() ──► ChatChunk stream             │
    │   │      │                        │                          │
    │   │      │               ModelToken events ──► CLI/TUI       │
    │   │      │                                                   │
    │   │      ▼                                                   │
    │   │  parse_action() ──► AgentAction                          │
    │   │      │                                                   │
    │   │      ├─ Answer ──────────────────► return to user        │
    │   │      │                                                   │
    │   │      ├─ ToolCall ──► policy.check()                      │
    │   │      │                 │                                 │
    │   │      │                 ├─ Allow ──► execute_tool_call()  │
    │   │      │                 │              │                  │
    │   │      │                 │              ▼                  │
    │   │      │                 │         ToolResult              │
    │   │      │                 │              │                  │
    │   │      │                 │              ▼                  │
    │   │      │                 │         AgentState.messages     │
    │   │      │                 │         transcript.append()     │
    │   │      │                 │                                 │
    │   │      │                 └─ Deny ──► error in state       │
    │   │      │                                                   │
    │   │      └─ NeedMoreContext ──► auto rg_search()             │
    │   │                               │                          │
    │   │                               ▼                          │
    │   │                         AgentState.context_entries        │
    │   │                                                          │
    │   └──────────────────────────────────────────────────────────┘
    │
    ▼
Final Answer (String)
```

---

## Extending agent-rs

### Adding a New Tool

1. Define input/output types in `crates/agent-tools/src/types.rs`.
2. Create the implementation in a new file under `crates/agent-tools/src/`.
3. Add the module to `crates/agent-tools/src/lib.rs`.
4. Register the tool in `create_default_registry()` in `registry.rs` with its JSON Schema.
5. Add the dispatch case in `crates/agent-core/src/executor.rs`.

### Adding a New Provider

1. Create a struct implementing `LlmProvider` in `crates/agent-provider/src/`.
2. Add the module to `crates/agent-provider/src/lib.rs`.
3. Add provider instantiation logic in the CLI (`crates/agent-cli/src/main.rs`) based on `config.provider.type`.

### Custom Tool Policy

1. Implement `ToolPolicy` in a new module.
2. Replace `DefaultPolicy::new()` with your policy in `agent_loop.rs`.

### Custom Frontend

Subscribe to the `EventBus` and consume `AgentEvent` values:

```rust
let event_bus = Arc::new(EventBus::default());
let mut rx = event_bus.subscribe();

tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        // Handle event
    }
});
```

---

## File-by-File Module Reference

### agent-core (10 files)

| File | Lines | Purpose |
|---|---|---|
| `lib.rs` | 10 | Module declarations |
| `agent_loop.rs` | ~300 | Main agent iteration loop |
| `config.rs` | ~135 | Configuration loading, defaults, env overrides |
| `context.rs` | ~135 | System prompt and message array assembly |
| `errors.rs` | ~35 | Typed error enum |
| `events.rs` | ~45 | Event bus and AgentEvent enum |
| `executor.rs` | ~165 | Tool call dispatch and policy integration |
| `planner.rs` | ~130 | Intent classification and search plan |
| `policy.rs` | ~110 | ToolPolicy trait and DefaultPolicy |
| `state.rs` | ~90 | Agent state accumulation |
| `types.rs` | ~55 | AgentMode, AgentTask, AgentAction, ToolCallId |

### agent-provider (5 files)

| File | Lines | Purpose |
|---|---|---|
| `lib.rs` | 5 | Module declarations |
| `provider.rs` | ~15 | LlmProvider trait definition |
| `types.rs` | ~95 | Chat request/response types |
| `ollama.rs` | ~100 | Ollama HTTP client with streaming |
| `openai_compat.rs` | ~45 | OpenAI-compatible stub |
| `streaming.rs` | ~50 | NDJSON stream parser |

### agent-tools (10 files)

| File | Lines | Purpose |
|---|---|---|
| `lib.rs` | 10 | Module declarations |
| `types.rs` | ~185 | All tool input/output types |
| `registry.rs` | ~200 | Tool registry with JSON Schema definitions |
| `rg.rs` | ~95 | Ripgrep search |
| `fs.rs` | ~100 | File read/write/tree |
| `git.rs` | ~110 | Git status/diff/commit/branch |
| `shell.rs` | ~85 | Allowlisted command execution |
| `patch.rs` | ~70 | Unified diff application |
| `project.rs` | ~80 | Project type detection |
| `quality.rs` | ~85 | Quality gate runner |
| `symbol.rs` | ~100 | Symbol extraction |

### agent-session (4 files)

| File | Lines | Purpose |
|---|---|---|
| `lib.rs` | 4 | Module declarations |
| `session.rs` | ~65 | Session struct and status enum |
| `store.rs` | ~130 | Session directory management |
| `transcript.rs` | ~85 | JSONL transcript append/read |
| `checkpoint.rs` | ~75 | Checkpoint save/load/list |

### agent-memory (4 files)

| File | Lines | Purpose |
|---|---|---|
| `lib.rs` | 4 | Module declarations |
| `short_term.rs` | ~90 | Priority-based in-memory store |
| `repo_cache.rs` | ~80 | Persisted repo metadata cache |
| `symbol_index.rs` | ~80 | Persisted symbol index |
| `summarizer.rs` | ~55 | LLM-based transcript summarization |

### agent-mcp (4 files)

| File | Lines | Purpose |
|---|---|---|
| `lib.rs` | 4 | Module declarations |
| `config.rs` | ~45 | Server and transport configuration |
| `types.rs` | ~30 | MCP tool and content types |
| `permissions.rs` | ~45 | Read/write permission model |
| `client.rs` | ~130 | MCP client lifecycle |

### agent-cli (1 file)

| File | Lines | Purpose |
|---|---|---|
| `main.rs` | ~320 | CLI entry point with all subcommands |

### agent-tui (7 files)

| File | Lines | Purpose |
|---|---|---|
| `main.rs` | ~50 | TUI entry point and terminal setup |
| `app.rs` | ~145 | Application state machine and event handling |
| `events.rs` | ~65 | Event source merging (crossterm + agent + tick) |
| `layout.rs` | ~55 | Layout computation |
| `theme.rs` | ~70 | Color scheme and style definitions |
| `widgets/mod.rs` | 5 | Widget module declarations |
| `widgets/chat.rs` | ~100 | Chat widget with streaming |
| `widgets/diff.rs` | ~65 | Colored diff widget |
| `widgets/files.rs` | ~45 | File list widget |
| `widgets/logs.rs` | ~70 | Log display widget |
| `widgets/status.rs` | ~70 | Status bar widget |

---

## Dependencies

All dependencies use standard open-source licenses (MIT, Apache-2.0, ISC, BSD):

| Category | Crate | Version | License | Purpose |
|---|---|---|---|---|
| Errors | `anyhow` | 1.x | MIT/Apache-2.0 | Flexible error handling |
| Errors | `thiserror` | 2.x | MIT/Apache-2.0 | Derive macro for error enums |
| Async | `tokio` | 1.x (full) | MIT | Async runtime |
| Async | `tokio-stream` | 0.1 | MIT | Stream utilities |
| Async | `futures` | 0.3 | MIT/Apache-2.0 | Future/stream combinators |
| Async | `async-trait` | 0.1 | MIT/Apache-2.0 | Async functions in traits |
| Serialization | `serde` | 1.x (derive) | MIT/Apache-2.0 | Serialization framework |
| Serialization | `serde_json` | 1.x | MIT/Apache-2.0 | JSON (de)serialization |
| Serialization | `toml` | 0.8 | MIT/Apache-2.0 | TOML config parsing |
| HTTP | `reqwest` | 0.12 (json, stream) | MIT/Apache-2.0 | HTTP client |
| CLI | `clap` | 4.x (derive) | MIT/Apache-2.0 | Argument parsing |
| Filesystem | `walkdir` | 2.x | MIT | Recursive directory traversal |
| Filesystem | `ignore` | 0.4 | MIT | Gitignore-aware traversal |
| Filesystem | `notify` | 8.x | CC0-1.0 | File system notifications |
| Diff | `similar` | 2.x | Apache-2.0 | Text diffing |
| Git | `git2` | 0.20 | MIT/Apache-2.0 | libgit2 bindings |
| Logging | `tracing` | 0.1 | MIT | Structured diagnostics |
| Logging | `tracing-subscriber` | 0.3 (env-filter) | MIT | Log output formatting |
| TUI | `ratatui` | 0.29 | MIT | Terminal UI framework |
| TUI | `crossterm` | 0.28 | MIT | Cross-platform terminal |
| MCP | `rmcp` | 0.1 (client, transports) | Apache-2.0/MIT | MCP protocol client |
| Time | `chrono` | 0.4 (serde) | MIT/Apache-2.0 | Date/time handling |
| IDs | `uuid` | 1.x (v4, serde) | MIT/Apache-2.0 | UUID generation |
| Streaming | `async-stream` | 0.3 | MIT | Async stream construction |

---

## Troubleshooting

### Ollama connection refused

```
Ollama API error 000: connection refused
```

Verify Ollama is running and accessible:

```bash
curl http://localhost:11434/api/tags
```

If running on a remote machine, ensure `OLLAMA_HOST=0.0.0.0:11434` is set and update `AGENT_PROVIDER_URL`.

### Model not found

```
Ollama API error 404: model "code-agent-14b" not found
```

Create the model:

```bash
ollama create code-agent-14b -f Modelfile
```

### ripgrep not found

```
tool error: rg_search: No such file or directory
```

Install ripgrep: `brew install ripgrep` (macOS), `apt install ripgrep` (Debian/Ubuntu), or `cargo install ripgrep`.

### git2 / libgit2 build failure

```
error: failed to compile `git2`
```

Install system dependencies:

- macOS: `xcode-select --install`
- Debian/Ubuntu: `apt install libssl-dev pkg-config cmake`
- Fedora: `dnf install openssl-devel cmake`

### Token budget exceeded

If the model produces incomplete or cut-off responses, increase `max_context_tokens` in the config. Note that this depends on the model's actual context window size (qwen2.5-coder:14b supports up to 32K tokens).

### Binary file rejection

```
binary file detected, refusing to read
```

The agent does not read binary files. This is by design. If you need the agent to work with a binary format, consider providing a text representation.

---

## Design Principles

- **Local-first.** No data leaves the local network. The agent communicates only with your Ollama instance. No API keys required.
- **Modular.** Each crate has a single responsibility and clear public API. Swap providers, tools, or policies independently.
- **Safe by default.** Write operations are gated by mode and policy. Push is never allowed. Sensitive files are protected. Commands are allowlisted.
- **Observable.** The event bus provides real-time visibility into agent behavior. Both CLI and TUI consume the same event stream. Custom frontends can subscribe too.
- **Resumable.** Sessions persist to disk with full transcripts and checkpoints. Sessions can be listed and resumed.
- **Deterministic.** Low temperature, structured JSON actions, typed tool inputs/outputs. The agent's behavior is reproducible and auditable via transcripts.
- **Clean-room.** No code copied from proprietary projects. All dependencies use standard open-source licenses.

---

## Project Status

### Functional

- Complete workspace structure (8 crates, 69 files, ~9000 lines).
- All quality gates pass: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo check --workspace`, `cargo test --workspace`.
- CLI binary with all 13 subcommands.
- Ollama provider with full NDJSON streaming.
- 13 local tools with typed I/O, security controls, and JSON Schema definitions.
- Unified tool registry (local + MCP).
- Agent loop with streaming, action parsing, and policy enforcement.
- Session persistence with JSONL transcripts and checkpoints.
- Short-term memory with priority eviction and file invalidation.
- Repo cache and symbol index with commit-based invalidation.
- TUI with multi-panel layout and event-driven updates.
- Tool policy system with mode-based access control.

### Stubbed (TODO)

- MCP server connections via RMCP transports (skeleton and config in place).
- OpenAI-compatible provider (trait implemented, method bodies are `todo!()`).
- Session resume state reconstruction from transcript.
- Summarizer integration into the agent loop.

---

## Roadmap

Planned improvements in priority order:

1. **MCP transport implementation** — Wire up RMCP stdio/SSE/HTTP transports for real MCP server connections.
2. **OpenAI-compatible provider** — Implement SSE streaming for `/v1/chat/completions`.
3. **Session resume** — Reconstruct `AgentState` from transcript entries on resume.
4. **Summarizer integration** — Auto-summarize when transcript exceeds token budget.
5. **Unit and integration tests** — Test coverage for tools, policy, context building, and action parsing.
6. **Confirmation prompts** — Interactive confirmation for write operations in Edit mode.
7. **Multi-file patch support** — Parse and apply multi-file unified diffs.
8. **Streaming tool results** — Stream large tool outputs incrementally.
9. **Watch mode** — File watcher that auto-triggers quality gates on changes.
10. **Plugin system** — Dynamic tool loading from external crates or shared libraries.

---

## Clean-Room Policy

- No code copied from any proprietary project.
- No dependencies on repositories subject to DMCA or legal disputes.
- Inspiration drawn only from publicly observable behaviors and published documentation.
- Every dependency license verified:
  - RMCP: Apache-2.0 / MIT
  - All other dependencies: MIT, Apache-2.0, ISC, BSD, or CC0

---

## License

All code in this repository is original work. Dependencies are listed above with their respective licenses.
