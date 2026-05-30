use ignore::WalkBuilder;
use std::path::Path;

use crate::types::{
    FileReadInput, FileReadOutput, FileWriteInput, FileWriteOutput, RepoTreeInput, RepoTreeOutput,
    TreeEntry,
};

const MAX_FILE_SIZE: u64 = 200 * 1024; // 200KB

/// List repository file tree respecting .gitignore.
pub fn repo_tree(input: &RepoTreeInput, repo_root: &Path) -> anyhow::Result<RepoTreeOutput> {
    let start = if let Some(ref p) = input.path {
        repo_root.join(p)
    } else {
        repo_root.to_path_buf()
    };

    let mut entries = Vec::new();

    let walker = WalkBuilder::new(&start)
        .max_depth(Some(input.max_depth))
        .hidden(true)
        .git_ignore(true)
        .git_global(false)
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if path == start {
            continue;
        }

        let relative = path
            .strip_prefix(repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let depth = entry.depth();
        let is_dir = path.is_dir();

        entries.push(TreeEntry {
            path: relative,
            is_dir,
            depth,
        });
    }

    Ok(RepoTreeOutput { entries })
}

/// Read a file with optional line range.
pub async fn file_read(input: &FileReadInput, repo_root: &Path) -> anyhow::Result<FileReadOutput> {
    let path = repo_root.join(&input.path);

    // Check file size
    let metadata = tokio::fs::metadata(&path).await?;
    if metadata.len() > MAX_FILE_SIZE {
        anyhow::bail!(
            "file too large: {} bytes (max {})",
            metadata.len(),
            MAX_FILE_SIZE
        );
    }

    // Reject binary files
    let content = tokio::fs::read(&path).await?;
    if content.iter().take(8192).any(|&b| b == 0) {
        anyhow::bail!("binary file detected, refusing to read");
    }

    let text = String::from_utf8_lossy(&content).to_string();
    let lines: Vec<&str> = text.lines().collect();
    let total_lines = lines.len();

    let start = input.start_line.unwrap_or(1).saturating_sub(1);
    let end = input.end_line.unwrap_or(total_lines).min(total_lines);

    let selected: String = lines[start..end].join("\n");

    Ok(FileReadOutput {
        path: input.path.clone(),
        content: selected,
        total_lines,
    })
}

/// Write content to a file.
pub async fn file_write(
    input: &FileWriteInput,
    repo_root: &Path,
) -> anyhow::Result<FileWriteOutput> {
    let path = repo_root.join(&input.path);

    // Ensure path is within repo
    let canonical_root = repo_root.canonicalize()?;
    if let Ok(canonical_path) = path.canonicalize() {
        if !canonical_path.starts_with(&canonical_root) {
            anyhow::bail!("path escapes repository root");
        }
    }

    if input.create_dirs {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }

    let bytes = input.content.as_bytes();
    tokio::fs::write(&path, bytes).await?;

    Ok(FileWriteOutput {
        path: input.path.clone(),
        bytes_written: bytes.len(),
    })
}
