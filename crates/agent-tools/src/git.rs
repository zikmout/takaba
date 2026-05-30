use std::path::Path;
use tokio::process::Command;

use crate::types::{
    GitCommitInput, GitCommitOutput, GitDiffOutput, GitFileStatus, GitStatusOutput,
};

/// Get git status.
pub async fn git_status(repo_root: &Path) -> anyhow::Result<GitStatusOutput> {
    let branch_output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(repo_root)
        .output()
        .await?;
    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    let status_output = Command::new("git")
        .args(["status", "--short"])
        .current_dir(repo_root)
        .output()
        .await?;
    let status_text = String::from_utf8_lossy(&status_output.stdout);

    let mut files = Vec::new();
    for line in status_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (status, path) = if line.len() > 3 {
            (line[..2].trim().to_string(), line[3..].to_string())
        } else {
            (line.to_string(), String::new())
        };
        files.push(GitFileStatus { path, status });
    }

    let clean = files.is_empty();

    Ok(GitStatusOutput {
        branch,
        files,
        clean,
    })
}

/// Get git diff (both staged and unstaged).
pub async fn git_diff(repo_root: &Path) -> anyhow::Result<GitDiffOutput> {
    let diff_output = Command::new("git")
        .args(["diff"])
        .current_dir(repo_root)
        .output()
        .await?;
    let diff = String::from_utf8_lossy(&diff_output.stdout).to_string();

    let staged_output = Command::new("git")
        .args(["diff", "--staged"])
        .current_dir(repo_root)
        .output()
        .await?;
    let staged_diff = String::from_utf8_lossy(&staged_output.stdout).to_string();

    Ok(GitDiffOutput { diff, staged_diff })
}

/// Create a git commit.
pub async fn git_commit(
    input: &GitCommitInput,
    repo_root: &Path,
) -> anyhow::Result<GitCommitOutput> {
    // Stage files
    let mut add_cmd = Command::new("git");
    add_cmd.arg("add").current_dir(repo_root);
    for file in &input.files {
        add_cmd.arg(file);
    }
    let add_output = add_cmd.output().await?;
    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        anyhow::bail!("git add failed: {stderr}");
    }

    // Commit
    let commit_output = Command::new("git")
        .args(["commit", "-m", &input.message])
        .current_dir(repo_root)
        .output()
        .await?;
    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        anyhow::bail!("git commit failed: {stderr}");
    }

    // Get commit hash
    let hash_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_root)
        .output()
        .await?;
    let commit_hash = String::from_utf8_lossy(&hash_output.stdout)
        .trim()
        .to_string();

    Ok(GitCommitOutput {
        commit_hash,
        message: input.message.clone(),
    })
}

/// Create and checkout a new branch.
pub async fn git_branch(name: &str, repo_root: &Path) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(["checkout", "-b", name])
        .current_dir(repo_root)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git checkout -b failed: {stderr}");
    }

    Ok(name.to_string())
}
