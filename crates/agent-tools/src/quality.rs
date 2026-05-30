use std::path::Path;
use tokio::process::Command;

use crate::types::{ProjectMetadata, QualityGateResult, QualityStep};

/// Run quality gate checks based on detected project type.
pub async fn run_quality_gate(
    project: &ProjectMetadata,
    repo_root: &Path,
) -> anyhow::Result<QualityGateResult> {
    let mut steps = Vec::new();

    match project.language.as_str() {
        "rust" => {
            steps.push(run_step("fmt", "cargo", &["fmt", "--check"], repo_root).await);
            steps.push(
                run_step(
                    "clippy",
                    "cargo",
                    &[
                        "clippy",
                        "--workspace",
                        "--all-targets",
                        "--",
                        "-D",
                        "warnings",
                    ],
                    repo_root,
                )
                .await,
            );
            steps.push(run_step("test", "cargo", &["test", "--workspace"], repo_root).await);
        }
        "typescript" | "javascript" => {
            let pm = project.package_manager.as_deref().unwrap_or("npm");
            if let Some(ref tc) = project.typecheck_command {
                let parts: Vec<&str> = tc.split_whitespace().collect();
                if parts.len() >= 2 {
                    steps.push(run_step("typecheck", parts[0], &parts[1..], repo_root).await);
                }
            }
            if let Some(ref lint) = project.lint_command {
                let parts: Vec<&str> = lint.split_whitespace().collect();
                if parts.len() >= 2 {
                    steps.push(run_step("lint", parts[0], &parts[1..], repo_root).await);
                }
            }
            if let Some(ref test) = project.test_command {
                let parts: Vec<&str> = test.split_whitespace().collect();
                if parts.len() >= 2 {
                    steps.push(run_step("test", parts[0], &parts[1..], repo_root).await);
                }
            }
            let _ = pm; // suppress unused warning
        }
        "go" => {
            steps.push(run_step("vet", "go", &["vet", "./..."], repo_root).await);
            steps.push(run_step("test", "go", &["test", "./..."], repo_root).await);
        }
        "python" => {
            if project.lint_command.is_some() {
                steps.push(run_step("lint", "ruff", &["check", "."], repo_root).await);
            }
            steps.push(run_step("test", "pytest", &[], repo_root).await);
        }
        _ => {
            tracing::warn!("no quality gate defined for language: {}", project.language);
        }
    }

    let passed = steps.iter().all(|s| s.passed);

    Ok(QualityGateResult { passed, steps })
}

async fn run_step(name: &str, cmd: &str, args: &[&str], cwd: &Path) -> QualityStep {
    let result = Command::new(cmd).args(args).current_dir(cwd).output().await;

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{stdout}{stderr}");
            let passed = output.status.success();

            QualityStep {
                name: name.to_string(),
                passed,
                output: truncate(&combined, 4096),
            }
        }
        Err(e) => QualityStep {
            name: name.to_string(),
            passed: false,
            output: format!("failed to execute: {e}"),
        },
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}... [truncated]", &s[..max])
    }
}
