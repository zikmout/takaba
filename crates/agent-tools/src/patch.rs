use std::path::Path;

use crate::types::{PatchInput, PatchOutput};

const MAX_PATCH_LINES: usize = 400;

const SENSITIVE_PATTERNS: &[&str] = &[
    ".env",
    ".pem",
    ".key",
    "id_rsa",
    "id_ed25519",
    "terraform.tfstate",
    ".secret",
    "credentials",
];

/// Apply a unified diff patch with dry-run support.
pub async fn apply_patch(input: &PatchInput, repo_root: &Path) -> anyhow::Result<PatchOutput> {
    let lines: Vec<&str> = input.diff.lines().collect();

    // Validate patch size
    if lines.len() > MAX_PATCH_LINES {
        anyhow::bail!(
            "patch too large: {} lines (max {MAX_PATCH_LINES})",
            lines.len()
        );
    }

    // Extract affected files and check for sensitive paths
    let mut files_changed = Vec::new();
    for line in &lines {
        if let Some(path) = line.strip_prefix("+++ b/") {
            let path = path.trim();
            for pattern in SENSITIVE_PATTERNS {
                if path.contains(pattern) {
                    anyhow::bail!("patch touches sensitive file: {path}");
                }
            }
            files_changed.push(path.to_string());
        }
    }

    if input.dry_run {
        return Ok(PatchOutput {
            applied: false,
            files_changed,
            dry_run: true,
        });
    }

    // Apply patch using git apply
    let mut cmd = tokio::process::Command::new("git");
    cmd.args(["apply", "--verbose", "-"])
        .current_dir(repo_root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(input.diff.as_bytes()).await?;
    }

    let output = child.wait_with_output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git apply failed: {stderr}");
    }

    Ok(PatchOutput {
        applied: true,
        files_changed,
        dry_run: false,
    })
}
