use gitai::{
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

    assert!(prompt.contains("- **Branch:** `feature/new-feature`"));
}

#[test]
fn test_create_user_prompt_includes_recent_commits() {
    let context = create_mock_commit_context();
    let prompt = create_user_prompt(&context, DetailLevel::Standard);

    // The mock context should have recent commits
    assert!(prompt.contains("- **Contextual History:**"));
    // Check that there's content after "Contextual History:"
    let recent_commits_start = prompt
        .find("- **Contextual History:**")
        .expect("Contextual History: should be in prompt");
    let after_start = &prompt[recent_commits_start + "- **Contextual History:**".len()..];
    assert!(!after_start.trim().is_empty());
}

#[test]
fn test_create_user_prompt_includes_staged_changes() {
    let context = create_mock_commit_context();
    let prompt = create_user_prompt(&context, DetailLevel::Standard);

    assert!(prompt.contains("- **Staged Change List:**"));
    // Check that it includes the mock file
    assert!(prompt.contains("file1.rs"));
}

#[test]
fn test_create_user_prompt_includes_detailed_changes() {
    let context = create_mock_commit_context();
    let prompt = create_user_prompt(&context, DetailLevel::Standard);

    assert!(prompt.contains("- **Detailed Diffs (Source of Truth):**"));
    // Should include the diff or analysis
    assert!(prompt.contains("CHANGE SUMMARY"));
}

#[test]
fn test_create_user_prompt_includes_author_history() {
    let context = create_mock_commit_context();
    let prompt = create_user_prompt(&context, DetailLevel::Standard);

    assert!(prompt.contains("- **Detected Style:**"));
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
    assert!(prompt.contains(&format!("- **Branch:** `{}`", context.branch)));

    // Test that recent commits are formatted with hash and message
    assert!(prompt.contains("- **Contextual History:**"));
    // Should contain the formatted commits
    for commit in &context.recent_commits {
        let expected_format = format!("{} - {}", &commit.hash[..7], commit.message);
        assert!(prompt.contains(&expected_format));
    }

    // Test staged changes formatting
    assert!(prompt.contains("- **Staged Change List:**"));
    for file in &context.staged_files {
        assert!(prompt.contains(&file.path));
    }

    // Test detailed changes formatting
    assert!(prompt.contains("- **Detailed Diffs (Source of Truth):**"));
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
        user_prompt.starts_with("### MAINTAINER TASK: GENERATE TECHNICAL COMMIT LOG"),
        "User prompt should start with task header"
    );
    assert!(
        user_prompt.contains("#### DATA CONTEXT"),
        "User prompt should have context information section"
    );
    assert!(
        user_prompt.contains("#### ANALYSIS REQUIREMENTS"),
        "User prompt should have analysis requirements section"
    );
    assert!(
        user_prompt.contains("RULES FOR SUCCESS"),
        "User prompt should contain RULES FOR SUCCESS instruction"
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
    assert!(
        prompt.contains("MAINTAINER TASK:"),
        "Should have task header"
    );
    assert!(
        prompt.contains("#### DATA CONTEXT"),
        "Should have context section"
    );
    assert!(
        prompt.contains("#### ANALYSIS REQUIREMENTS"),
        "Should have analysis requirements section"
    );

    // Test bold formatting
    assert!(prompt.contains("**Branch:**"), "Should use bold for branch");
    assert!(
        prompt.contains("**Contextual History:**"),
        "Should use bold for contextual history"
    );
    assert!(
        prompt.contains("**Staged Change List:**"),
        "Should use bold for staged changes"
    );

    // Test analysis requirements
    assert!(
        prompt.contains("1. **Subsystem Subject:**"),
        "Should have subsystem subject instruction"
    );
}

#[test]
fn test_create_user_prompt_respects_detail_level() {
    let context = create_mock_commit_context();

    let minimal = create_user_prompt(&context, DetailLevel::Minimal);
    assert!(minimal.contains("Keep it technical and concise"));
    assert!(minimal.contains("subsystem subject"));

    let standard = create_user_prompt(&context, DetailLevel::Standard);
    assert!(standard.contains("multi-paragraph technical justification"));
    assert!(standard.contains("problem and solution"));

    let detailed = create_user_prompt(&context, DetailLevel::Detailed);
    assert!(detailed.contains("Exhaustive technical documentation"));
    assert!(detailed.contains("logic flow"));
    assert!(detailed.contains("architectural implications"));
}

#[test]
fn test_commit_system_prompt_structure() {
    let config = create_mock_config();
    let prompt = create_system_prompt(&config).expect("Failed to create system prompt");

    // Test persona definition
    assert!(prompt.contains("# PERSONA"), "Should have persona header");
    assert!(
        prompt.contains("Linux Kernel Maintainer"),
        "Should define persona"
    );

    // Test structured sections
    assert!(
        prompt.contains("# OPERATIONAL GUIDELINES"),
        "Should have operational guidelines section"
    );
    assert!(prompt.contains("# TASK"), "Should have task section");
    assert!(
        prompt.contains("# OUTPUT SPECIFICATION"),
        "Should have output specification"
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
        prompt.contains("### TASK: COMPLETE PARTIAL COMMIT MESSAGE"),
        "Should have completion task header"
    );

    // Test user input section
    assert!(
        prompt.contains("#### USER INPUT"),
        "Should have user input section"
    );
    assert!(
        prompt.contains("**Current Prefix:**"),
        "Should have bold prefix label"
    );

    // Test context information section
    assert!(
        prompt.contains("#### DATA CONTEXT"),
        "Should have context information section"
    );

    // Test completion requirements
    assert!(
        prompt.contains("#### COMPLETION INSTRUCTIONS"),
        "Should have completion instructions section"
    );
    assert!(
        prompt.contains("1. **Syntactic Match:**"),
        "Should have numbered completion steps"
    );
    assert!(
        prompt.contains("2. **Pattern Recognition:**"),
        "Should have numbered completion steps"
    );
}

#[test]
fn test_completion_system_prompt_structure() {
    let config = create_mock_config();
    let prompt = create_completion_system_prompt(&config)
        .expect("Failed to create completion system prompt");

    // Test persona definition
    assert!(prompt.contains("# PERSONA"), "Should have persona header");
    assert!(
        prompt.contains("Git Workflow Expert"),
        "Should define persona"
    );

    // Test structured sections
    assert!(
        prompt.contains("# OPERATIONAL GUIDELINES"),
        "Should have guidelines section"
    );
    assert!(prompt.contains("# TASK"), "Should have task section");
    assert!(
        prompt.contains("# OUTPUT SPECIFICATION"),
        "Should have output specification"
    );

    // Test numbered guidelines
    assert!(
        prompt.contains("1. **Contextual Continuity:**"),
        "Should have numbered completion rules"
    );
    assert!(
        prompt.contains("2. **Zero Redundancy:**"),
        "Should have numbered completion rules"
    );
}

#[test]
fn test_pr_system_prompt_structure() {
    let config = create_mock_config();
    let prompt = create_pr_system_prompt(&config).expect("Failed to create PR system prompt");

    // Test persona definition
    assert!(prompt.contains("# PERSONA"), "Should have persona header");
    assert!(
        prompt.contains("Staff Technical Writer"),
        "Should define persona"
    );

    // Test structured sections
    assert!(
        prompt.contains("# CORE OBJECTIVE"),
        "Should have objective section"
    );
    assert!(
        prompt.contains("# ANALYTICAL PROTOCOL"),
        "Should have analytical protocol section"
    );
    assert!(
        prompt.contains("# PR ANATOMY"),
        "Should have anatomy section"
    );

    // Test numbered protocol
    assert!(
        prompt.contains("1. **Holistic Synthesis:**"),
        "Should have numbered protocol"
    );
}
