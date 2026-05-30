use std::path::Path;

use crate::types::ProjectMetadata;

/// Detect project type by checking for common config files.
pub fn detect_project(repo_root: &Path) -> anyhow::Result<ProjectMetadata> {
    // Rust
    if repo_root.join("Cargo.toml").exists() {
        return Ok(ProjectMetadata {
            language: "rust".into(),
            package_manager: Some("cargo".into()),
            test_command: Some("cargo test".into()),
            lint_command: Some("cargo clippy -- -D warnings".into()),
            typecheck_command: Some("cargo check".into()),
            build_command: Some("cargo build".into()),
            root: repo_root.to_path_buf(),
        });
    }

    // Node.js (pnpm preferred)
    if repo_root.join("package.json").exists() {
        let pm = if repo_root.join("pnpm-lock.yaml").exists() {
            "pnpm"
        } else if repo_root.join("yarn.lock").exists() {
            "yarn"
        } else {
            "npm"
        };

        return Ok(ProjectMetadata {
            language: "typescript".into(),
            package_manager: Some(pm.into()),
            test_command: Some(format!("{pm} test")),
            lint_command: Some(format!("{pm} lint")),
            typecheck_command: Some(format!("{pm} typecheck")),
            build_command: Some(format!("{pm} build")),
            root: repo_root.to_path_buf(),
        });
    }

    // Go
    if repo_root.join("go.mod").exists() {
        return Ok(ProjectMetadata {
            language: "go".into(),
            package_manager: Some("go".into()),
            test_command: Some("go test ./...".into()),
            lint_command: Some("golangci-lint run".into()),
            typecheck_command: Some("go vet ./...".into()),
            build_command: Some("go build ./...".into()),
            root: repo_root.to_path_buf(),
        });
    }

    // Python
    if repo_root.join("pyproject.toml").exists() || repo_root.join("setup.py").exists() {
        return Ok(ProjectMetadata {
            language: "python".into(),
            package_manager: Some("pip".into()),
            test_command: Some("pytest".into()),
            lint_command: Some("ruff check .".into()),
            typecheck_command: Some("mypy .".into()),
            build_command: None,
            root: repo_root.to_path_buf(),
        });
    }

    // Fallback
    Ok(ProjectMetadata {
        language: "unknown".into(),
        package_manager: None,
        test_command: None,
        lint_command: None,
        typecheck_command: None,
        build_command: None,
        root: repo_root.to_path_buf(),
    })
}
