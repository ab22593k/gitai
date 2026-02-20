use gitai::{
    commit::prompt::{create_system_prompt, create_user_prompt},
    context::ChangeType,
};

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{MockDataBuilder, TestAssertions};

#[test]
fn test_create_user_prompt_basic() {
    let commit_context = MockDataBuilder::commit_context();

    let prompt = create_user_prompt(&commit_context);

    TestAssertions::assert_commit_prompt_essentials(&prompt);
    assert!(prompt.contains("Initial commit"));
    assert!(prompt.contains("file1.rs"));
    assert!(prompt.contains("Modified"));
}

#[test]
fn test_create_user_prompt_with_staged_files() {
    let commit_context = MockDataBuilder::commit_context();

    let prompt = create_user_prompt(&commit_context);

    TestAssertions::assert_commit_prompt_essentials(&prompt);
    assert!(prompt.contains("file1.rs"));
    assert!(prompt.contains("Modified"));
    assert!(prompt.contains("- old line\n+ new line"));
}

#[test]
fn test_create_system_prompt_with_emoji() {
    let config = MockDataBuilder::config_with_emoji();

    let prompt = create_system_prompt(&config).expect("Failed to create system prompt");

    TestAssertions::assert_contains_emoji(&prompt);
}

#[test]
fn test_create_system_prompt_with_custom_instructions() {
    let config = MockDataBuilder::config_with_instructions("Always mention the ticket number");

    let prompt = create_system_prompt(&config).expect("Failed to create system prompt");

    assert!(prompt.contains("Always mention the ticket number"));
}

#[test]
fn test_create_user_prompt_verbose() {
    let commit_context = MockDataBuilder::commit_context();

    let prompt = create_user_prompt(&commit_context);

    assert!(prompt.contains("Detailed changes"));
}

#[test]
fn test_create_user_prompt() {
    let commit_context = MockDataBuilder::commit_context();

    let prompt = create_user_prompt(&commit_context);

    TestAssertions::assert_commit_prompt_essentials(&prompt);
    assert!(prompt.contains("Initial commit"));
    assert!(prompt.contains("file1.rs"));
    assert!(prompt.contains("Modified"));
    assert!(prompt.contains("- old line\n+ new line"));
}

#[test]
fn test_create_user_prompt_with_multiple_staged_files() {
    let mut commit_context = MockDataBuilder::commit_context();

    // Add another staged file using our builder
    commit_context
        .staged_files
        .push(MockDataBuilder::staged_file_with(
            "file2.rs",
            ChangeType::Added,
            "+ new file content",
            vec!["New function: helper".to_string()],
        ));

    let prompt = create_user_prompt(&commit_context);

    assert!(prompt.contains("file1.rs"));
    assert!(prompt.contains("Modified"));
    assert!(prompt.contains("file2.rs"));
    assert!(prompt.contains("Added"));
    assert!(prompt.contains("- old line\n+ new line"));
    assert!(prompt.contains("+ new file content"));
}

#[test]
fn test_create_user_prompt_with_project_metadata() {
    let mut commit_context = MockDataBuilder::commit_context();
    commit_context.project_metadata = MockDataBuilder::project_metadata_with(
        Some("Rust".to_string()),
        Some("Rocket".to_string()),
        vec!["serde".to_string(), "tokio".to_string()],
    );

    let prompt = create_user_prompt(&commit_context);

    assert!(prompt.contains("Language: Rust"));
    assert!(prompt.contains("Framework: Rocket"));
    assert!(prompt.contains("Dependencies: serde, tokio"));
}

#[test]
fn test_create_user_prompt_with_file_analysis() {
    let mut commit_context = MockDataBuilder::commit_context();
    commit_context.staged_files[0].analysis = vec![
        "Modified function: main".to_string(),
        "Added new struct: User".to_string(),
    ];

    let prompt = create_user_prompt(&commit_context);

    assert!(prompt.contains("Modified function: main"));
    assert!(prompt.contains("Added new struct: User"));
}
