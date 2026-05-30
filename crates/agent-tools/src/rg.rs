use std::path::Path;
use tokio::process::Command;

use crate::types::{RgMatch, RgSearchInput, RgSearchOutput};

/// Execute a ripgrep search and return structured results.
pub async fn rg_search(input: &RgSearchInput, repo_root: &Path) -> anyhow::Result<RgSearchOutput> {
    let mut cmd = Command::new("rg");
    cmd.arg("--json")
        .arg("--max-count")
        .arg(input.max_results.to_string());

    if !input.case_sensitive {
        cmd.arg("--ignore-case");
    }

    if input.context_lines > 0 {
        cmd.arg("-C").arg(input.context_lines.to_string());
    }

    if let Some(glob) = &input.glob {
        cmd.arg("--glob").arg(glob);
    }

    cmd.arg(&input.pattern).current_dir(repo_root);

    let output = cmd.output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut matches = Vec::new();
    let mut context_before: Vec<String> = Vec::new();
    let mut context_after: Vec<String> = Vec::new();
    let mut last_match: Option<RgMatch> = None;

    for line in stdout.lines() {
        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
            let entry_type = entry["type"].as_str().unwrap_or("");

            match entry_type {
                "match" => {
                    // Flush previous match
                    if let Some(mut m) = last_match.take() {
                        m.context_before = context_before.clone();
                        matches.push(m);
                    }
                    context_before.clear();
                    context_after.clear();

                    let data = &entry["data"];
                    let path = data["path"]["text"].as_str().unwrap_or("").to_string();
                    let line_number = data["line_number"].as_u64().unwrap_or(0) as usize;
                    let line_content = data["lines"]["text"]
                        .as_str()
                        .unwrap_or("")
                        .trim_end()
                        .to_string();

                    last_match = Some(RgMatch {
                        path,
                        line_number,
                        line_content,
                        context_before: Vec::new(),
                        context_after: Vec::new(),
                    });
                }
                "context" => {
                    let data = &entry["data"];
                    let text = data["lines"]["text"]
                        .as_str()
                        .unwrap_or("")
                        .trim_end()
                        .to_string();

                    if last_match.is_some() {
                        context_after.push(text);
                    } else {
                        context_before.push(text);
                    }
                }
                _ => {}
            }
        }
    }

    // Flush last match
    if let Some(mut m) = last_match.take() {
        m.context_before = context_before;
        m.context_after = context_after;
        matches.push(m);
    }

    let truncated = matches.len() >= input.max_results;

    Ok(RgSearchOutput { matches, truncated })
}
