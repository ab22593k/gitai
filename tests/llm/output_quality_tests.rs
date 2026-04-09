//! Golden-file tests for LLM output format stability.
//!
//! These tests verify that the *structure* and *formatting* of generated
//! commit messages and PR descriptions remain stable across prompt changes.
//! They catch regressions where an LLM prompt modification causes the output
//! to deviate from the expected JSON schema or formatting contract.
//!
//! Each golden file contains a representative `GeneratedMessage` or
//! `GeneratedPullRequest` serialized as JSON. The test deserializes it,
//! formats it, and asserts on structural invariants.

use gitai::commands::commit::types::{
    GeneratedMessage, GeneratedPullRequest, format_commit_message, format_pull_request,
};

// ============================================================================
// Commit Message Golden Files
// ============================================================================

const GOLDEN_COMMIT_MESSAGE: &str = r#"{
  "title": "feat(llm): add provider enum dispatch for model routing",
  "message": "Replace hardcoded provider strings scattered across engine.rs,\nconfig.rs, and model_info.rs with a shared ProviderKind enum.\n\nThe enum bridges our internal provider identity to the external\nllm crate's LLMBackend type, enabling static dispatch with zero\nvtable overhead.\n\nCerebras maps to OpenRouter backend since the llm crate doesn't\nsupport it natively yet."
}"#;

#[test]
fn test_commit_message_structure() {
    let msg: GeneratedMessage =
        serde_json::from_str(GOLDEN_COMMIT_MESSAGE).expect("Golden commit message is valid JSON");

    assert!(!msg.title.is_empty());
    assert!(
        msg.title.len() <= 72,
        "Title should be short enough for git log"
    );
    assert!(
        !msg.message.is_empty(),
        "Body should contain meaningful content"
    );
}

#[test]
fn test_commit_message_formatting() {
    let msg: GeneratedMessage =
        serde_json::from_str(GOLDEN_COMMIT_MESSAGE).expect("valid golden commit message");

    let formatted = format_commit_message(&msg);
    let lines: Vec<&str> = formatted.lines().collect();

    // Title is first line
    assert_eq!(lines[0], msg.title);

    // Blank separator line
    assert_eq!(lines[1], "");

    // Body lines respect 72-char wrap
    for line in &lines[2..] {
        assert!(
            line.len() <= 72,
            "Line exceeds 72 chars: '{line}' ({})",
            line.len()
        );
    }
}

#[test]
fn test_commit_message_title_prefix_patterns() {
    // Verify common conventional commit patterns are handled correctly
    let test_cases = [
        ("feat: add new feature", "feat scope"),
        ("fix: resolve null panic", "fix scope"),
        ("chore(deps): update dependencies", "chore scope"),
        ("refactor(llm): extract provider enum", "refactor scope"),
        ("docs: add architecture decision records", "docs scope"),
    ];

    for (title, _label) in test_cases {
        let msg = GeneratedMessage {
            title: title.to_string(),
            message: "Body content here.".to_string(),
        };
        let formatted = format_commit_message(&msg);
        assert!(
            formatted.starts_with(&msg.title),
            "Title '{title}' should appear at start of formatted message"
        );
    }
}

// ============================================================================
// Pull Request Golden Files
// ============================================================================

const GOLDEN_PR_DESCRIPTION: &str = r#"{
  "title": "feat: consolidate provider strings into shared enum",
  "summary": "Replaces 37 hardcoded provider string literals across 7 files with a single ProviderKind enum in src/llm/provider.rs.",
  "description": "This change improves maintainability and reduces the risk of typos or missed updates when adding new LLM providers.\n\nKey changes:\n- New ProviderKind enum with display names, backend mapping, and default models\n- Updated engine.rs, config.rs, model_info.rs, and command modules\n- Removed async_trait from model_info.rs (dyn compatibility barrier removed)",
  "commits": [
    "feat(llm): add provider enum dispatch for model routing",
    "refactor(config): use ProviderKind for provider lookups",
    "fix(clippy): resolve match_same_arms and redundant_closure"
  ],
  "breaking_changes": [],
  "testing_notes": "All 239 tests pass. Clippy clean.",
  "notes": "Cerebras backend maps to OpenRouter since the llm crate doesn't support it yet."
}"#;

#[test]
fn test_pr_description_structure() {
    let pr: GeneratedPullRequest =
        serde_json::from_str(GOLDEN_PR_DESCRIPTION).expect("Golden PR is valid JSON");

    assert!(!pr.title.is_empty());
    assert!(!pr.summary.is_empty());
    assert!(!pr.description.is_empty());
    assert!(!pr.commits.is_empty(), "PR should reference commits");
}

#[test]
fn test_pr_description_formatting() {
    let pr: GeneratedPullRequest =
        serde_json::from_str(GOLDEN_PR_DESCRIPTION).expect("valid golden PR description");

    let formatted = format_pull_request(&pr);

    // Markdown sections present
    assert!(formatted.contains("## Summary"));
    assert!(formatted.contains("## Description"));
    assert!(formatted.contains("## Commits"));
    assert!(formatted.contains("## Testing"));
    assert!(formatted.contains("## Notes"));

    // Commits listed as bullet list
    for commit in &pr.commits {
        assert!(
            formatted.contains(&format!("- {commit}")),
            "Commit '{commit}' should appear as bullet point"
        );
    }

    // Breaking changes section empty → omitted
    assert!(
        !formatted.contains("## Breaking Changes"),
        "Empty breaking_changes should omit the section"
    );
}

#[test]
fn test_pr_with_breaking_changes() {
    let pr = GeneratedPullRequest {
        title: "feat: rename project gait to gitai".to_string(),
        summary: "Project renamed across all config files, binaries, and docs.".to_string(),
        description: "The project is now called gitai.".to_string(),
        commits: vec!["chore: rename project to gitai".to_string()],
        breaking_changes: vec![
            "Config file paths changed from `gitai.*` to `gitai.*` in git config".to_string(),
            "Binary names updated from `git-*` prefix to `git-*`".to_string(),
        ],
        testing_notes: None,
        notes: None,
    };

    let formatted = format_pull_request(&pr);
    assert!(formatted.contains("## Breaking Changes"));
    for change in &pr.breaking_changes {
        assert!(formatted.contains(&format!("- {change}")));
    }
}

#[test]
fn test_pr_optional_fields_omitted() {
    let pr = GeneratedPullRequest {
        title: "fix: typo in readme".to_string(),
        summary: "Fixed a typo.".to_string(),
        description: "Self-explanatory.".to_string(),
        commits: vec![],
        breaking_changes: vec![],
        testing_notes: None,
        notes: None,
    };

    let formatted = format_pull_request(&pr);

    // Empty sections omitted
    assert!(!formatted.contains("## Commits"));
    assert!(!formatted.contains("## Breaking Changes"));
    assert!(!formatted.contains("## Testing"));
    assert!(!formatted.contains("## Notes"));
}
