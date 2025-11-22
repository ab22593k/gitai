use gitai::{
    config::Config,
    core::context::CommitContext,
    features::commit::prompt::{create_system_prompt, create_user_prompt},
};

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::MockDataBuilder;

/// Creates a mock configuration for testing
fn create_mock_config() -> Config {
    MockDataBuilder::config_with_instructions("Write clear, concise commit messages")
}

/// Creates a mock commit context for testing
fn create_mock_commit_context() -> CommitContext {
    MockDataBuilder::commit_context()
}

#[test]
fn test_create_user_prompt_formats_branch_correctly() {
    let mut context = create_mock_commit_context();
    context.branch = "feature/new-feature".to_string();
    let prompt = create_user_prompt(&context);

    assert!(prompt.contains("Branch (feature/new-feature)"));
}

#[test]
fn test_create_user_prompt_includes_recent_commits() {
    let context = create_mock_commit_context();
    let prompt = create_user_prompt(&context);

    // The mock context should have recent commits
    assert!(prompt.contains("Recent Commits"));
    // Since we can't easily check the exact format without duplicating logic,
    // we check that it's not empty after "Recent Commits ("
    let recent_commits_start = prompt
        .find("Recent Commits (")
        .expect("Recent Commits ( should be in prompt");
    let after_start = &prompt[recent_commits_start + "Recent Commits (".len()..];
    assert!(!after_start.starts_with(')'));
}

#[test]
fn test_create_user_prompt_includes_staged_changes() {
    let context = create_mock_commit_context();
    let prompt = create_user_prompt(&context);

    assert!(prompt.contains("Staged Changes"));
    // Check that it includes the mock file
    assert!(prompt.contains("file1.rs"));
}

#[test]
fn test_create_user_prompt_includes_detailed_changes() {
    let context = create_mock_commit_context();
    let prompt = create_user_prompt(&context);

    assert!(prompt.contains("Detailed Changes"));
    // Should include the diff or analysis
    assert!(prompt.contains("CHANGE SUMMARY"));
}

#[test]
fn test_create_user_prompt_includes_author_history() {
    let context = create_mock_commit_context();
    let prompt = create_user_prompt(&context);

    assert!(prompt.contains("Author's Commit History"));
    // Mock should have some history
    assert!(prompt.contains("feat: add user authentication"));
}

#[test]
fn test_system_prompt_includes_valid_json_schema() {
    let config = create_mock_config();
    let prompt = create_system_prompt(&config).expect("Failed to create system prompt");

    // Extract the schema part from the prompt
    let schema_start = prompt
        .find("designed for structured data extraction:")
        .expect("schema marker should be in prompt");
    let schema_part = &prompt[schema_start + "designed for structured data extraction:".len()..];

    // The schema should be valid JSON
    // Verify it contains the expected GeneratedMessage fields
    assert!(schema_part.contains("\"title\""));
    assert!(schema_part.contains("\"message\""));
    assert!(schema_part.contains("\"type\""));
    assert!(schema_part.contains("\"string\""));
}

#[test]
fn test_user_prompt_context_elements_are_properly_formatted() {
    let context = create_mock_commit_context();
    let prompt = create_user_prompt(&context);

    // Test branch formatting
    assert!(prompt.contains(&format!("Branch ({})", context.branch)));

    // Test that recent commits are formatted with hash and message
    assert!(prompt.contains("Recent Commits ("));
    // Should contain the formatted commits
    for commit in &context.recent_commits {
        let expected_format = format!("{} - {}", &commit.hash[..7], commit.message);
        assert!(prompt.contains(&expected_format));
    }

    // Test staged changes formatting
    assert!(prompt.contains("Staged Changes ("));
    for file in &context.staged_files {
        assert!(prompt.contains(&file.path));
        // Should contain relevance score
        assert!(prompt.contains("(2.00)")); // Relevance score
    }

    // Test detailed changes formatting
    assert!(prompt.contains("Detailed Changes ("));
    assert!(prompt.contains("CHANGE SUMMARY"));
    assert!(prompt.contains("file(s) added"));
    assert!(prompt.contains("file(s) modified"));
    assert!(prompt.contains("file(s) deleted"));
    assert!(prompt.contains("=== DIFFS"));
}

#[test]
fn test_combined_prompt_structure_for_llm() {
    let config = create_mock_config();
    let context = create_mock_commit_context();

    let system_prompt = create_system_prompt(&config).expect("Failed to create system prompt");
    let user_prompt = create_user_prompt(&context);

    // Test that both prompts are non-empty and well-formed
    assert!(!system_prompt.is_empty());
    assert!(!user_prompt.is_empty());

    // System prompt should end with schema
    assert!(system_prompt.contains("GeneratedMessage"));

    // User prompt should start with analysis instruction
    assert!(user_prompt.starts_with("ANALYZE"));

    // Sanity check exact prompt lengths
    assert_eq!(system_prompt.len(), 1003);
    assert_eq!(user_prompt.len(), 660);
}
