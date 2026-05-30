use std::path::Path;
use std::time::Duration;
use tokio::process::Command;

use crate::types::{ShellCommandInput, ShellCommandOutput};

const MAX_OUTPUT_BYTES: usize = 64 * 1024; // 64KB

const ALLOWLIST: &[&str] = &[
    "cargo", "npm", "pnpm", "yarn", "go", "python", "python3", "pytest", "make", "docker", "git",
    "rg", "ls", "cat", "head", "tail", "wc", "sort", "uniq", "grep", "find", "tree",
];

const BLOCKLIST_PATTERNS: &[&str] = &[
    "rm -rf",
    "curl|sh",
    "wget|sh",
    "sudo",
    "chmod 777",
    "dd ",
    "mkfs",
    "nc ",
    "nmap",
    "ssh ",
    "scp ",
    "> /dev/",
    ":(){ :",
];

/// Run a command with allowlist enforcement and timeout.
pub async fn run_command(
    input: &ShellCommandInput,
    repo_root: &Path,
) -> anyhow::Result<ShellCommandOutput> {
    // Check allowlist
    if !ALLOWLIST.contains(&input.command.as_str()) {
        anyhow::bail!(
            "command '{}' not in allowlist: {:?}",
            input.command,
            ALLOWLIST
        );
    }

    // Check blocklist
    let full_cmd = format!("{} {}", input.command, input.args.join(" "));
    for pattern in BLOCKLIST_PATTERNS {
        if full_cmd.contains(pattern) {
            anyhow::bail!("blocked command pattern detected: {pattern}");
        }
    }

    let timeout = Duration::from_secs(input.timeout_secs);

    let result = tokio::time::timeout(timeout, async {
        Command::new(&input.command)
            .args(&input.args)
            .current_dir(repo_root)
            .output()
            .await
    })
    .await;

    match result {
        Ok(Ok(output)) => {
            let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let mut truncated = false;

            if stdout.len() > MAX_OUTPUT_BYTES {
                stdout.truncate(MAX_OUTPUT_BYTES);
                stdout.push_str("\n... [truncated]");
                truncated = true;
            }
            if stderr.len() > MAX_OUTPUT_BYTES {
                stderr.truncate(MAX_OUTPUT_BYTES);
                stderr.push_str("\n... [truncated]");
                truncated = true;
            }

            Ok(ShellCommandOutput {
                stdout,
                stderr,
                exit_code: output.status.code().unwrap_or(-1),
                truncated,
            })
        }
        Ok(Err(e)) => anyhow::bail!("command execution failed: {e}"),
        Err(_) => anyhow::bail!("command timed out after {}s", input.timeout_secs),
    }
}
