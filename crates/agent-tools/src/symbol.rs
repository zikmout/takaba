use std::path::Path;
use tokio::process::Command;

use crate::types::{Symbol, SymbolKind};

/// Index symbols in the codebase using ripgrep.
pub async fn symbol_index(repo_root: &Path, glob: Option<&str>) -> anyhow::Result<Vec<Symbol>> {
    let patterns = [
        (r"pub\s+fn\s+(\w+)", SymbolKind::Function),
        (r"fn\s+(\w+)", SymbolKind::Function),
        (r"pub\s+struct\s+(\w+)", SymbolKind::Struct),
        (r"struct\s+(\w+)", SymbolKind::Struct),
        (r"pub\s+enum\s+(\w+)", SymbolKind::Enum),
        (r"enum\s+(\w+)", SymbolKind::Enum),
        (r"pub\s+trait\s+(\w+)", SymbolKind::Trait),
        (r"trait\s+(\w+)", SymbolKind::Trait),
        (r"impl\s+(\w+)", SymbolKind::Impl),
        (r"function\s+(\w+)", SymbolKind::Function),
        (r"class\s+(\w+)", SymbolKind::Class),
        (r"interface\s+(\w+)", SymbolKind::Interface),
        (r"export\s+function\s+(\w+)", SymbolKind::Function),
        (r"export\s+class\s+(\w+)", SymbolKind::Class),
        (r"export\s+interface\s+(\w+)", SymbolKind::Interface),
    ];

    let mut symbols = Vec::new();

    for (pattern, kind) in &patterns {
        let mut cmd = Command::new("rg");
        cmd.arg("--json")
            .arg("--no-heading")
            .arg("-e")
            .arg(pattern)
            .current_dir(repo_root);

        if let Some(g) = glob {
            cmd.arg("--glob").arg(g);
        }

        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
                if entry["type"].as_str() != Some("match") {
                    continue;
                }

                let data = &entry["data"];
                let file = data["path"]["text"].as_str().unwrap_or("").to_string();
                let line_num = data["line_number"].as_u64().unwrap_or(0) as usize;
                let line_text = data["lines"]["text"].as_str().unwrap_or("");

                // Extract symbol name from the match
                if let Some(name) = extract_symbol_name(line_text, *kind) {
                    symbols.push(Symbol {
                        name,
                        kind: *kind,
                        file,
                        line: line_num,
                    });
                }
            }
        }
    }

    // Deduplicate by (name, file, line)
    symbols.sort_by(|a, b| (&a.file, a.line, &a.name).cmp(&(&b.file, b.line, &b.name)));
    symbols.dedup_by(|a, b| a.file == b.file && a.line == b.line && a.name == b.name);

    Ok(symbols)
}

fn extract_symbol_name(line: &str, kind: SymbolKind) -> Option<String> {
    let keywords: &[&str] = match kind {
        SymbolKind::Function => &["fn ", "function "],
        SymbolKind::Struct => &["struct "],
        SymbolKind::Enum => &["enum "],
        SymbolKind::Trait => &["trait "],
        SymbolKind::Impl => &["impl "],
        SymbolKind::Class => &["class "],
        SymbolKind::Interface => &["interface "],
        _ => return None,
    };

    for kw in keywords {
        if let Some(pos) = line.find(kw) {
            let after = &line[pos + kw.len()..];
            let name: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }

    None
}
