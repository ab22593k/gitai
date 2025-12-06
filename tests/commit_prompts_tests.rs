use gait::{
    common::DetailLevel,
    config::Config,
    core::context::CommitContext,
    features::commit::prompt::{
        create_completion_system_prompt, create_completion_user_prompt, create_pr_system_prompt,
        create_system_prompt, create_user_prompt,
    },
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
    let prompt = create_user_prompt(&context, DetailLevel::Standard);

    assert!(prompt.contains("**Branch:** feature/new-feature"));
}

#[test]
fn test_create_user_prompt_includes_recent_commits() {
    let context = create_mock_commit_context();
    let prompt = create_user_prompt(&context, DetailLevel::Standard);

    // The mock context should have recent commits
    assert!(prompt.contains("Recent Commits:"));
    // Check that there's content after "Recent Commits:"
    let recent_commits_start = prompt
        .find("Recent Commits:")
        .expect("Recent Commits: should be in prompt");
    let after_start = &prompt[recent_commits_start + "Recent Commits:".len()..];
    assert!(!after_start.trim().is_empty());
}

#[test]
fn test_create_user_prompt_includes_staged_changes() {
    let context = create_mock_commit_context();
    let prompt = create_user_prompt(&context, DetailLevel::Standard);

    assert!(prompt.contains("Staged Changes List"));
    // Check that it includes the mock file
    assert!(prompt.contains("file1.rs"));
}

#[test]
fn test_create_user_prompt_includes_detailed_changes() {
    let context = create_mock_commit_context();
    let prompt = create_user_prompt(&context, DetailLevel::Standard);

    assert!(prompt.contains("Detailed Changes (Diffs)"));
    // Should include the diff or analysis
    assert!(prompt.contains("CHANGE SUMMARY"));
}

#[test]
fn test_create_user_prompt_includes_author_history() {
    let context = create_mock_commit_context();
    let prompt = create_user_prompt(&context, DetailLevel::Standard);

    assert!(prompt.contains("Author's Commit History"));
    // Mock should have some history
    assert!(prompt.contains("feat: add user authentication"));
}

#[test]
fn test_system_prompt_includes_valid_json_schema() {
    let config = create_mock_config();
    let prompt = create_system_prompt(&config).expect("Failed to create system prompt");

    // Extract the schema part from the prompt (now in JSON code block)
    let schema_start = prompt
        .find("```json")
        .expect("JSON code block should be in prompt");
    let schema_part = &prompt[schema_start + "```json".len()..];

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
    let prompt = create_user_prompt(&context, DetailLevel::Standard);

    // Test branch formatting
    assert!(prompt.contains(&format!("**Branch:** {}", context.branch)));

    // Test that recent commits are formatted with hash and message
    assert!(prompt.contains("Recent Commits:"));
    // Should contain the formatted commits
    for commit in &context.recent_commits {
        let expected_format = format!("{} - {}", &commit.hash[..7], commit.message);
        assert!(prompt.contains(&expected_format));
    }

    // Test staged changes formatting
    assert!(prompt.contains("Staged Changes List:"));
    for file in &context.staged_files {
        assert!(prompt.contains(&file.path));
        // Should contain relevance score
        assert!(prompt.contains("(2.00)")); // Relevance score
    }

    // Test detailed changes formatting
    assert!(prompt.contains("Detailed Changes (Diffs):"));
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
    let user_prompt = create_user_prompt(&context, DetailLevel::Standard);

    // Test that both prompts are non-empty and well-formed
    assert!(!system_prompt.is_empty());
    assert!(!user_prompt.is_empty());

    // System prompt should end with schema
    assert!(system_prompt.contains("GeneratedMessage"));

    // User prompt should have structured format
    assert!(
        user_prompt.starts_with("# TASK: Generate Commit Message"),
        "User prompt should start with task header"
    );
    assert!(
        user_prompt.contains("## Context Information"),
        "User prompt should have context information section"
    );
    assert!(
        user_prompt.contains("## Analysis Requirements"),
        "User prompt should have analysis requirements section"
    );
    assert!(
        user_prompt.contains("ANALYZE"),
        "User prompt should contain ANALYZE instruction"
    );

    // Sanity check prompt lengths (will be different due to improved formatting)
    assert!(
        system_prompt.len() > 1000,
        "System prompt should be substantial"
    );
    assert!(user_prompt.len() > 600, "User prompt should be substantial");
}

#[test]
fn test_commit_user_prompt_structure() {
    let context = create_mock_commit_context();
    let prompt = create_user_prompt(&context, DetailLevel::Standard);

    // Test markdown structure
    assert!(prompt.contains("# TASK:"), "Should have task header");
    assert!(
        prompt.contains("## Context Information"),
        "Should have context section"
    );
    assert!(
        prompt.contains("## Analysis Requirements"),
        "Should have analysis section"
    );

    // Test bold formatting
    assert!(prompt.contains("**Branch:**"), "Should use bold for branch");
    assert!(
        prompt.contains("**Recent Commits:**"),
        "Should use bold for recent commits"
    );
    assert!(
        prompt.contains("**Staged Changes List:**"),
        "Should use bold for staged changes"
    );

    // Test numbered requirements
    assert!(
        prompt.contains("1. **PRIMARY FOCUS:**"),
        "Should have primary focus instruction"
    );
    assert!(
        prompt.contains("2. ANALYZE"),
        "Should have numbered analysis steps"
    );
}

#[test]
fn test_create_user_prompt_respects_detail_level() {
    let context = create_mock_commit_context();

    let minimal = create_user_prompt(&context, DetailLevel::Minimal);
    assert!(minimal.contains("Make the message EXTREMELY concise"));
    assert!(minimal.contains("Generate ONLY a single title line"));

    let standard = create_user_prompt(&context, DetailLevel::Standard);
    assert!(standard.contains("Make the message concise yet descriptive"));

    let detailed = create_user_prompt(&context, DetailLevel::Detailed);
    assert!(detailed.contains("Provide a detailed explanation"));
    assert!(detailed.contains("detailed bullet points explaining the changes"));
}

#[test]
fn test_commit_system_prompt_structure() {
    let config = create_mock_config();
    let prompt = create_system_prompt(&config).expect("Failed to create system prompt");

    // Test role definition
    assert!(prompt.contains("# ROLE:"), "Should have role header");
    assert!(
        prompt.contains("Git Commit Message Generator"),
        "Should define generator role"
    );

    // Test structured sections
    assert!(
        prompt.contains("## Core Responsibilities"),
        "Should have responsibilities section"
    );
    assert!(
        prompt.contains("## Instructions"),
        "Should have instructions section"
    );
    assert!(
        prompt.contains("## Output Requirements"),
        "Should have output requirements"
    );

    // Test numbered responsibilities
    assert!(
        prompt.contains("1. **Analyze Context:**"),
        "Should have numbered responsibilities"
    );
    assert!(
        prompt.contains("2. **Generate Messages:**"),
        "Should have numbered responsibilities"
    );

    // Test JSON schema formatting
    assert!(prompt.contains("```json"), "Should have JSON code block");
    assert!(prompt.contains("```"), "Should close JSON code block");
}

#[test]
fn test_completion_user_prompt_structure() {
    let context = create_mock_commit_context();
    let prefix = "feat: add user";
    let context_ratio = 0.5;

    let prompt = create_completion_user_prompt(&context, prefix, context_ratio);

    // Test task header
    assert!(
        prompt.contains("# TASK: Complete Commit Message"),
        "Should have completion task header"
    );

    // Test message prefix section
    assert!(
        prompt.contains("## Message Prefix"),
        "Should have message prefix section"
    );
    assert!(
        prompt.contains("**Prefix:**"),
        "Should have bold prefix label"
    );

    // Test context information section
    assert!(
        prompt.contains("## Context Information"),
        "Should have context information section"
    );

    // Test completion requirements
    assert!(
        prompt.contains("## Completion Requirements"),
        "Should have completion requirements section"
    );
    assert!(
        prompt.contains("1. ANALYZE"),
        "Should have numbered completion steps"
    );
    assert!(
        prompt.contains("2. Complete"),
        "Should have numbered completion steps"
    );
}

#[test]
fn test_completion_system_prompt_structure() {
    let config = create_mock_config();
    let prompt = create_completion_system_prompt(&config)
        .expect("Failed to create completion system prompt");

    // Test role definition
    assert!(prompt.contains("# ROLE:"), "Should have role header");
    assert!(
        prompt.contains("Git Commit Message Completion Specialist"),
        "Should define completion specialist role"
    );

    // Test structured sections
    assert!(
        prompt.contains("## Core Responsibilities"),
        "Should have responsibilities section"
    );
    assert!(
        prompt.contains("## Completion Rules"),
        "Should have completion rules section"
    );
    assert!(
        prompt.contains("## Output Requirements"),
        "Should have output requirements"
    );

    // Test numbered rules
    assert!(
        prompt.contains("1. **Start Point:**"),
        "Should have numbered completion rules"
    );
    assert!(
        prompt.contains("2. **Style Consistency:**"),
        "Should have numbered completion rules"
    );
}

#[test]
fn test_pr_system_prompt_structure() {
    let config = create_mock_config();
    let prompt = create_pr_system_prompt(&config).expect("Failed to create PR system prompt");

    // Test role definition
    assert!(prompt.contains("# ROLE:"), "Should have role header");
    assert!(
        prompt.contains("Pull Request Description Specialist"),
        "Should define PR specialist role"
    );

    // Test structured sections
    assert!(
        prompt.contains("## Core Responsibilities"),
        "Should have responsibilities section"
    );
    assert!(
        prompt.contains("## PR Description Structure"),
        "Should have PR structure section"
    );
    assert!(
        prompt.contains("## Guidelines"),
        "Should have guidelines section"
    );

    // Test numbered structure
    assert!(
        prompt.contains("1. **Title:**"),
        "Should have numbered PR structure"
    );
    assert!(
        prompt.contains("2. **Summary:**"),
        "Should have numbered PR structure"
    );

    // Test guidelines
    assert!(
        prompt.contains("- **Holistic View:**"),
        "Should have structured guidelines"
    );
    assert!(
        prompt.contains("- **Clear Language:**"),
        "Should have structured guidelines"
    );
}
