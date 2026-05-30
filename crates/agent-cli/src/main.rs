use std::path::Path;
use std::sync::Arc;

use agent_core::agent_loop::AgentLoop;
use agent_core::config::AgentConfig;
use agent_core::events::{AgentEvent, EventBus};
use agent_core::types::{AgentMode, AgentTask};
use agent_provider::ollama::OllamaProvider;
use agent_provider::provider::LlmProvider;
use agent_session::store::SessionStore;
use agent_tools::registry::create_default_registry;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "agent", about = "Local code agent powered by Ollama")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Read-only query mode
    Ask {
        #[arg(trailing_var_arg = true)]
        prompt: Vec<String>,
    },
    /// Produce a plan without modifying files
    Plan {
        #[arg(trailing_var_arg = true)]
        prompt: Vec<String>,
    },
    /// May read, search, and modify files
    Edit {
        #[arg(trailing_var_arg = true)]
        prompt: Vec<String>,
    },
    /// Full autonomous mode
    Auto {
        #[arg(trailing_var_arg = true)]
        prompt: Vec<String>,
    },
    /// Run quality gate checks
    Test,
    /// Show git status + session info
    Status,
    /// Show git diff
    Diff,
    /// Create a git commit
    Commit {
        #[arg(short, long)]
        message: String,
    },
    /// Session management
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },
    /// List MCP tools
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },
    /// List available providers
    Provider {
        #[command(subcommand)]
        command: ProviderCommands,
    },
    /// Show current configuration
    Config,
}

#[derive(Subcommand)]
enum SessionCommands {
    /// List all sessions
    List,
    /// Resume a session
    Resume { id: String },
}

#[derive(Subcommand)]
enum McpCommands {
    /// List MCP tools
    List,
}

#[derive(Subcommand)]
enum ProviderCommands {
    /// List providers
    List,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let working_dir = std::env::current_dir()?;
    let config = AgentConfig::load(&working_dir)?;

    match cli.command {
        Commands::Ask { prompt } => {
            run_agent(prompt.join(" "), AgentMode::Ask, &config, &working_dir).await?;
        }
        Commands::Plan { prompt } => {
            run_agent(prompt.join(" "), AgentMode::Plan, &config, &working_dir).await?;
        }
        Commands::Edit { prompt } => {
            run_agent(prompt.join(" "), AgentMode::Edit, &config, &working_dir).await?;
        }
        Commands::Auto { prompt } => {
            run_agent(prompt.join(" "), AgentMode::Auto, &config, &working_dir).await?;
        }
        Commands::Test => {
            let project = agent_tools::project::detect_project(&working_dir)?;
            println!("Running quality gate for {} project...", project.language);
            let result = agent_tools::quality::run_quality_gate(&project, &working_dir).await?;
            for step in &result.steps {
                let icon = if step.passed { "PASS" } else { "FAIL" };
                println!("[{icon}] {}", step.name);
                if !step.passed {
                    println!(
                        "  {}",
                        step.output.lines().take(5).collect::<Vec<_>>().join("\n  ")
                    );
                }
            }
            if result.passed {
                println!("\nAll checks passed.");
            } else {
                println!("\nSome checks failed.");
                std::process::exit(1);
            }
        }
        Commands::Status => {
            let status = agent_tools::git::git_status(&working_dir).await?;
            println!("Branch: {}", status.branch);
            if status.clean {
                println!("Working tree clean");
            } else {
                for f in &status.files {
                    println!("  {} {}", f.status, f.path);
                }
            }
        }
        Commands::Diff => {
            let diff = agent_tools::git::git_diff(&working_dir).await?;
            if !diff.staged_diff.is_empty() {
                println!("=== Staged ===");
                println!("{}", diff.staged_diff);
            }
            if !diff.diff.is_empty() {
                println!("=== Unstaged ===");
                println!("{}", diff.diff);
            }
            if diff.diff.is_empty() && diff.staged_diff.is_empty() {
                println!("No changes");
            }
        }
        Commands::Commit { message } => {
            let status = agent_tools::git::git_status(&working_dir).await?;
            let files: Vec<String> = status.files.iter().map(|f| f.path.clone()).collect();
            if files.is_empty() {
                println!("Nothing to commit");
                return Ok(());
            }
            let input = agent_tools::types::GitCommitInput { message, files };
            let result = agent_tools::git::git_commit(&input, &working_dir).await?;
            println!(
                "Committed: {} ({})",
                result.message,
                &result.commit_hash[..8]
            );
        }
        Commands::Session { command } => match command {
            SessionCommands::List => {
                let agent_dir = AgentConfig::agent_dir(&working_dir);
                let store = SessionStore::new(&agent_dir);
                let sessions = store.list().await?;
                if sessions.is_empty() {
                    println!("No sessions found");
                } else {
                    for s in &sessions {
                        println!(
                            "{} [{:?}] {} - {}",
                            s.id,
                            s.status,
                            s.created_at.format("%Y-%m-%d %H:%M"),
                            truncate_str(&s.prompt, 50)
                        );
                    }
                }
            }
            SessionCommands::Resume { id } => {
                println!("Session resume not yet fully implemented for: {id}");
            }
        },
        Commands::Mcp { command } => match command {
            McpCommands::List => {
                let registry = create_default_registry();
                println!("Registered tools:");
                for tool in registry.list_tools() {
                    println!("  {} - {}", tool.name(), tool.description());
                }
            }
        },
        Commands::Provider { command } => match command {
            ProviderCommands::List => {
                println!("Available providers:");
                println!(
                    "  ollama        - Ollama local inference (current: {})",
                    config.provider.base_url
                );
                println!("  openai_compat - OpenAI-compatible API (not configured)");
            }
        },
        Commands::Config => {
            let toml_str = toml::to_string_pretty(&config)?;
            println!("{toml_str}");
        }
    }

    Ok(())
}

async fn run_agent(
    prompt: String,
    mode: AgentMode,
    config: &AgentConfig,
    working_dir: &Path,
) -> anyhow::Result<()> {
    if prompt.is_empty() {
        anyhow::bail!("prompt cannot be empty");
    }

    let provider: Arc<dyn LlmProvider> = Arc::new(OllamaProvider::new(
        config.provider.base_url.clone(),
        config.provider.model.clone(),
    ));

    let registry = create_default_registry();
    let event_bus = Arc::new(EventBus::default());
    let mut rx = event_bus.subscribe();

    let task = AgentTask {
        prompt,
        mode,
        working_dir: working_dir.to_path_buf(),
    };

    // Spawn event printer
    let print_handle = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            match event {
                AgentEvent::SessionStarted(id) => {
                    eprintln!("[session] started: {id}");
                }
                AgentEvent::IterationStarted(i) => {
                    eprintln!("[iteration] {i}");
                }
                AgentEvent::ModelToken(token) => {
                    eprint!("{token}");
                }
                AgentEvent::ModelResponseComplete(_) => {
                    eprintln!();
                }
                AgentEvent::ToolCallStarted(_, name) => {
                    eprintln!("[tool] {name}...");
                }
                AgentEvent::ToolCallFinished(_, name) => {
                    eprintln!("[tool] {name} done");
                }
                AgentEvent::PatchApplied(msg) => {
                    eprintln!("[patch] {msg}");
                }
                AgentEvent::QualityGateStarted => {
                    eprintln!("[quality] running...");
                }
                AgentEvent::QualityGateFinished(passed) => {
                    eprintln!("[quality] {}", if passed { "passed" } else { "failed" });
                }
                AgentEvent::SessionCompleted => {
                    eprintln!("[session] completed");
                }
                AgentEvent::Error(msg) => {
                    eprintln!("[error] {msg}");
                }
                _ => {}
            }
        }
    });

    let agent_loop = AgentLoop::new(
        config.clone(),
        provider,
        registry,
        event_bus,
        working_dir.to_path_buf(),
    );

    let answer = agent_loop.run(task).await?;
    println!("\n{answer}");

    // Give event printer a moment to flush
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    print_handle.abort();

    Ok(())
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
