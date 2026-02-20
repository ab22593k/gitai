use gitai::{
    changes::{
        change_analyzer::AnalyzedChange,
        models::ChangeMetrics,
        prompt::{
            create_changelog_system_prompt, create_changelog_user_prompt,
            create_release_notes_system_prompt, create_release_notes_user_prompt,
        },
    },
    common::DetailLevel,
    config::Config,
};

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::MockDataBuilder;

/// Creates a mock configuration for testing
fn create_mock_config() -> Config {
    MockDataBuilder::config_with_instructions("Always mention performance impacts")
}

/// Creates a mock analyzed change for testing
fn create_mock_analyzed_change() -> AnalyzedChange {
    MockDataBuilder::analyzed_change()
}

/// Creates mock total metrics for testing
fn create_mock_total_metrics() -> ChangeMetrics {
    MockDataBuilder::total_change_metrics()
}

#[test]
fn test_create_changelog_system_prompt() {
    let config = create_mock_config();
    let prompt = create_changelog_system_prompt(&config);

    // Assert that the prompt contains key instructions and elements
    assert!(prompt.contains("You are an AI assistant specialized in generating clear, concise, and informative changelogs"));
    assert!(prompt.contains("Always mention performance impacts"));
    assert!(
        prompt.contains("Ensure that your response is a valid JSON object matching this structure")
    );
    assert!(prompt.contains("ChangelogResponse"));
    assert!(prompt.contains("sections"));
    assert!(prompt.contains("breaking_changes"));
    assert!(prompt.contains("metrics"));
}

#[test]
fn test_create_changelog_user_prompt_minimal() {
    let changes = vec![create_mock_analyzed_change()];
    let total_metrics = create_mock_total_metrics();
    let readme_summary = Some("This project is a fantastic tool for managing workflows.");

    let minimal_prompt = create_changelog_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Minimal,
        "v1.0.0",
        "v1.1.0",
        readme_summary,
    );

    // Test basic structure
    assert!(minimal_prompt.contains("Based on the following changes from v1.0.0 to v1.1.0"));
    assert!(minimal_prompt.contains("Overall Changes:"));

    // Test metrics
    assert!(minimal_prompt.contains("Total commits: 5"));
    assert!(minimal_prompt.contains("Files changed: 10"));
    assert!(minimal_prompt.contains("Total lines changed: 150"));
    assert!(minimal_prompt.contains("Insertions: 100"));
    assert!(minimal_prompt.contains("Deletions: 50"));

    // Test change details
    assert!(minimal_prompt.contains("Commit: abcdef123456"));
    assert!(minimal_prompt.contains("Author: Jane Doe"));
    assert!(minimal_prompt.contains("Message: Add new feature"));
    assert!(minimal_prompt.contains("Type: Added"));
    assert!(minimal_prompt.contains("Breaking Change: false"));
    assert!(minimal_prompt.contains("Associated Issues: #123"));
    assert!(minimal_prompt.contains("Pull Request: PR #456"));
    assert!(minimal_prompt.contains("Impact score: 0.75"));

    // Test minimal detail level specific behavior
    assert!(!minimal_prompt.contains("File changes:"));
    assert!(minimal_prompt.contains("Please generate a concise changelog"));

    // Test README summary inclusion
    assert!(minimal_prompt.contains("Project README Summary:"));
    assert!(minimal_prompt.contains("This project is a fantastic tool for managing workflows."));
}

#[test]
fn test_create_changelog_user_prompt_standard() {
    let changes = vec![create_mock_analyzed_change()];
    let total_metrics = create_mock_total_metrics();
    let readme_summary = Some("This project is a fantastic tool for managing workflows.");

    let standard_prompt = create_changelog_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Standard,
        "v1.0.0",
        "v1.1.0",
        readme_summary,
    );

    assert!(standard_prompt.contains("File changes:"));
    assert!(standard_prompt.contains("src/new.rs (Modified)"));
    assert!(standard_prompt.contains("Please generate a comprehensive changelog"));
}

#[test]
fn test_create_changelog_user_prompt_detailed() {
    let changes = vec![create_mock_analyzed_change()];
    let total_metrics = create_mock_total_metrics();
    let readme_summary = Some("This project is a fantastic tool for managing workflows.");

    let detailed_prompt = create_changelog_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Detailed,
        "v1.0.0",
        "v1.1.0",
        readme_summary,
    );

    assert!(detailed_prompt.contains("File changes:"));
    assert!(detailed_prompt.contains("src/new.rs (Modified)"));
    assert!(detailed_prompt.contains("Modified function: process_data"));
    assert!(detailed_prompt.contains("Please generate a highly detailed changelog"));
}

#[test]
fn test_create_release_notes_system_prompt() {
    let config = create_mock_config();
    let prompt = create_release_notes_system_prompt(&config);

    // Assert that the prompt contains key instructions and elements
    assert!(prompt.contains("You are an AI assistant specialized in generating comprehensive and user-friendly release notes"));
    assert!(prompt.contains("Always mention performance impacts"));
    assert!(
        prompt.contains("Ensure that your response is a valid JSON object matching this structure")
    );
    assert!(prompt.contains("ReleaseNotesResponse"));
    assert!(prompt.contains("sections"));
    assert!(prompt.contains("breaking_changes"));
    assert!(prompt.contains("metrics"));
}

#[test]
fn test_create_release_notes_user_prompt() {
    let changes = vec![create_mock_analyzed_change()];
    let total_metrics = create_mock_total_metrics();
    let readme_summary = Some("This project is a fantastic tool for managing workflows.");

    // Test Minimal detail level
    let minimal_prompt = create_release_notes_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Minimal,
        "v1.0.0",
        "v1.1.0",
        readme_summary,
    );
    assert!(minimal_prompt.contains("Based on the following changes from v1.0.0 to v1.1.0"));
    assert!(minimal_prompt.contains("generate concise release notes"));
    assert!(minimal_prompt.contains("Keep the release notes brief"));
    assert!(minimal_prompt.contains("Project README Summary:"));
    assert!(minimal_prompt.contains("This project is a fantastic tool for managing workflows."));

    // Test Standard detail level
    let standard_prompt = create_release_notes_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Standard,
        "v1.0.0",
        "v1.1.0",
        readme_summary,
    );
    assert!(standard_prompt.contains("generate comprehensive release notes"));
    assert!(standard_prompt.contains("Provide a balanced overview"));

    // Test Detailed detail level
    let detailed_prompt = create_release_notes_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Detailed,
        "v1.0.0",
        "v1.1.0",
        readme_summary,
    );
    assert!(detailed_prompt.contains("generate highly detailed release notes"));
    assert!(detailed_prompt.contains("Include detailed explanations"));
}

#[test]
fn test_changelog_user_prompt_without_readme() {
    let changes = vec![create_mock_analyzed_change()];
    let total_metrics = create_mock_total_metrics();
    let prompt = create_changelog_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Standard,
        "v1.0.0",
        "v1.1.0",
        None,
    );

    assert!(!prompt.contains("Project README Summary:"));
    assert!(prompt.contains("Based on the following changes from v1.0.0 to v1.1.0"));
    assert!(prompt.contains("generate a comprehensive changelog"));
}

#[test]
fn test_release_notes_user_prompt_without_readme() {
    let changes = vec![create_mock_analyzed_change()];
    let total_metrics = create_mock_total_metrics();
    let prompt = create_release_notes_user_prompt(
        &changes,
        &total_metrics,
        DetailLevel::Standard,
        "v1.0.0",
        "v1.1.0",
        None,
    );

    assert!(!prompt.contains("Project README Summary:"));
    assert!(prompt.contains("Based on the following changes from v1.0.0 to v1.1.0"));
    assert!(prompt.contains("generate comprehensive release notes"));
}
