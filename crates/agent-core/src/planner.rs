use crate::types::AgentMode;

/// Simple intent classification based on mode and prompt analysis.
pub fn classify_intent(prompt: &str, mode: AgentMode) -> Intent {
    match mode {
        AgentMode::Ask => Intent::Query,
        AgentMode::Plan => Intent::Plan,
        AgentMode::Edit => {
            if needs_search(prompt) {
                Intent::SearchThenEdit
            } else {
                Intent::DirectEdit
            }
        }
        AgentMode::Auto => {
            if needs_search(prompt) {
                Intent::SearchThenEdit
            } else {
                Intent::DirectEdit
            }
        }
    }
}

/// Generate a search plan based on the prompt.
pub fn generate_search_plan(prompt: &str) -> Vec<SearchStep> {
    let mut steps = Vec::new();

    // Always start with project detection
    steps.push(SearchStep::DetectProject);

    // Check git status
    steps.push(SearchStep::GitStatus);

    // Try to find relevant files
    let keywords = extract_keywords(prompt);
    for kw in &keywords {
        steps.push(SearchStep::Search(kw.clone()));
    }

    // If the prompt mentions specific files
    let file_hints = extract_file_hints(prompt);
    for file in file_hints {
        steps.push(SearchStep::ReadFile(file));
    }

    steps
}

fn needs_search(prompt: &str) -> bool {
    let search_indicators = [
        "find",
        "search",
        "look",
        "where",
        "which",
        "what",
        "how",
        "bug",
        "fix",
        "error",
        "issue",
        "refactor",
        "update",
        "change",
        "modify",
        "add",
        "implement",
        "create",
    ];

    let lower = prompt.to_lowercase();
    search_indicators.iter().any(|kw| lower.contains(kw))
}

fn extract_keywords(prompt: &str) -> Vec<String> {
    // Extract meaningful words (>3 chars, not common stop words)
    let stop_words = [
        "the", "this", "that", "with", "from", "have", "been", "will", "would", "could", "should",
        "about", "there", "their", "which", "what", "when", "where", "they", "them", "then",
        "than", "these", "those", "some", "other", "into", "more", "very", "just", "also", "here",
        "only", "each", "such", "like", "make", "made", "does",
    ];

    prompt
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|w| w.len() > 3 && !stop_words.contains(&w.as_str()))
        .take(5)
        .collect()
}

fn extract_file_hints(prompt: &str) -> Vec<String> {
    prompt
        .split_whitespace()
        .filter(|w| {
            w.contains('.')
                && (w.ends_with(".rs")
                    || w.ends_with(".ts")
                    || w.ends_with(".py")
                    || w.ends_with(".go")
                    || w.ends_with(".js")
                    || w.ends_with(".toml")
                    || w.ends_with(".json")
                    || w.ends_with(".yaml")
                    || w.ends_with(".yml"))
        })
        .map(|w| {
            w.trim_matches(|c: char| {
                !c.is_alphanumeric() && c != '.' && c != '/' && c != '_' && c != '-'
            })
            .to_string()
        })
        .collect()
}

#[derive(Debug, Clone)]
pub enum Intent {
    Query,
    Plan,
    DirectEdit,
    SearchThenEdit,
}

#[derive(Debug, Clone)]
pub enum SearchStep {
    DetectProject,
    GitStatus,
    Search(String),
    ReadFile(String),
}
