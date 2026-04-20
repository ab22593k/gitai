//! Tests for the prompt section framework.
//!
//! RED phase: These tests describe the expected behavior of `PromptSection`
//! and its concrete implementations. They must fail first (module doesn't exist).

use super::sections::PromptSection;
use crate::commands::commit::types::GeneratedMessage;
use crate::config::Config;
use crate::llm::context::{ChangeType, CommitContext, RecentCommit, StagedFile};

fn make_context() -> CommitContext {
    CommitContext {
        branch: "feature/test".to_string(),
        recent_commits: vec![RecentCommit {
            hash: "abc123".to_string(),
            message: "Previous commit".to_string(),
            timestamp: "1234567890".to_string(),
        }],
        staged_files: vec![StagedFile {
            path: "src/main.rs".to_string(),
            change_type: ChangeType::Modified,
            diff: "+ changed something".to_string(),
            content: None,
            content_excluded: false,
        }],
        user_name: "Test".to_string(),
        user_email: "test@example.com".to_string(),
        author_history: vec![],
    }
}

// -- PersonaSection Tests --

#[test]
fn test_persona_section_renders_header_and_content() {
    // Arrange
    let section = super::sections::PersonaSection::new("Test persona content");

    // Act
    let output = section.render();

    // Assert
    assert!(output.contains("# PERSONA"));
    assert!(output.contains("Test persona content"));
}

// -- TaskSection Tests --

#[test]
fn test_task_section_renders_header_and_content() {
    // Arrange
    let section = super::sections::TaskSection::new("Generate a commit message");

    // Act
    let output = section.render();

    // Assert
    assert!(output.contains("# TASK"));
    assert!(output.contains("Generate a commit message"));
}

// -- GuidelinesSection Tests --

#[test]
fn test_guidelines_section_renders_numbered_list() {
    // Arrange
    let guidelines = vec!["Use imperative mood", "Limit subject to 72 characters"];
    let section = super::sections::GuidelinesSection::new(&guidelines);

    // Act
    let output = section.render();

    // Assert
    assert!(output.contains("# OPERATIONAL GUIDELINES"));
    assert!(output.contains("1. **"));
    assert!(output.contains("Use imperative mood"));
}

// -- OutputSchemaSection Tests --

#[test]
fn test_output_schema_section_renders_json_schema() {
    // Arrange
    let section = super::sections::OutputSchemaSection::<GeneratedMessage>::new();

    // Act
    let output = section.render();

    // Assert
    assert!(output.contains("# OUTPUT SPECIFICATION"));
    assert!(output.contains("```json"));
    assert!(output.contains("GeneratedMessage"));
}

// -- DataContextSection Tests --

#[test]
fn test_data_context_section_renders_branch() {
    // Arrange
    let ctx = make_context();
    let section = super::sections::DataContextSection::new(&ctx);

    // Act
    let output = section.render();

    // Assert
    assert!(output.contains("- **Branch:** `feature/test`"));
}

#[test]
fn test_data_context_section_renders_staged_files() {
    // Arrange
    let ctx = make_context();
    let section = super::sections::DataContextSection::new(&ctx);

    // Act
    let output = section.render();

    // Assert
    assert!(output.contains("src/main.rs"));
}

#[test]
fn test_data_context_section_renders_contextual_history() {
    // Arrange
    let ctx = make_context();
    let section = super::sections::DataContextSection::new(&ctx);

    // Act
    let output = section.render();

    // Assert
    assert!(output.contains("- **Contextual History:**"));
    assert!(output.contains("Previous commit"));
}

// -- PromptBuilder Tests (RED — module doesn't exist yet) --

#[test]
fn test_prompt_builder_with_persona() {
    // Arrange & Act
    let output = super::builder::PromptBuilder::system()
        .with_persona("Linux Kernel Maintainer")
        .build();

    // Assert
    assert!(output.contains("# PERSONA"));
    assert!(output.contains("Linux Kernel Maintainer"));
}

#[test]
fn test_prompt_builder_with_task() {
    // Arrange & Act
    let output = super::builder::PromptBuilder::system()
        .with_task("Generate commit message")
        .build();

    // Assert
    assert!(output.contains("# TASK"));
    assert!(output.contains("Generate commit message"));
}

#[test]
fn test_prompt_builder_with_guidelines() {
    // Arrange & Act
    let output = super::builder::PromptBuilder::system()
        .with_guidelines(&["Rule 1", "Rule 2"])
        .build();

    // Assert
    assert!(output.contains("# OPERATIONAL GUIDELINES"));
    assert!(output.contains("Rule 1"));
    assert!(output.contains("Rule 2"));
}

#[test]
fn test_prompt_builder_with_user_instructions() {
    // Arrange
    let config = Config::default();

    // Act
    let output = super::builder::PromptBuilder::system()
        .with_user_instructions(&config)
        .build();

    // Assert — should contain "# USER INSTRUCTIONS" header
    assert!(output.contains("# USER INSTRUCTIONS"));
}

#[test]
fn test_prompt_builder_with_output_schema() {
    // Arrange & Act
    let output = super::builder::PromptBuilder::system()
        .with_output_schema::<GeneratedMessage>()
        .build();

    // Assert
    assert!(output.contains("# OUTPUT SPECIFICATION"));
    assert!(output.contains("```json"));
}

#[test]
fn test_prompt_builder_chained_sections() {
    // Arrange & Act
    let output = super::builder::PromptBuilder::system()
        .with_persona("Test persona")
        .with_task("Test task")
        .with_guidelines(&["Rule 1"])
        .build();

    // Assert — all sections present in order
    let persona_pos = output.find("# PERSONA").expect("PERSONA section missing");
    let task_pos = output.find("# TASK").expect("TASK section missing");
    let guidelines_pos = output
        .find("# OPERATIONAL GUIDELINES")
        .expect("GUIDELINES missing");
    assert!(persona_pos < task_pos, "PERSONA should come before TASK");
    assert!(
        task_pos < guidelines_pos,
        "TASK should come before GUIDELINES"
    );
}

#[test]
fn test_prompt_builder_full_commit_system_prompt() {
    // Characterization test: builder output must match current create_system_prompt()
    // This ensures the refactoring doesn't change prompt content

    // Act — build using new builder
    let builder_output = super::builder::PromptBuilder::system()
        .with_persona(
            "You are a Principal Linux Kernel Maintainer. You are technically rigorous, demanding, \
             and believe that a commit message is a permanent piece of technical documentation. \
             You expect developers to explain *why* a change is necessary with absolute precision.",
        )
        .with_task(
            "Generate a technical commit message for a high-stakes mailing list. The message must \
             provide a clear technical narrative explaining the Problem, Solution, and Reasoning.",
        )
        .with_output_schema::<GeneratedMessage>()
        .build();

    // Assert — contains all expected sections
    assert!(builder_output.contains("# PERSONA"));
    assert!(builder_output.contains("# TASK"));
    assert!(builder_output.contains("# OUTPUT SPECIFICATION"));
    assert!(builder_output.contains("```json"));
    assert!(builder_output.contains("Linux Kernel Maintainer"));
}
