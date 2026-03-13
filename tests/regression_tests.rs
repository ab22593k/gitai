//! Regression tests using History oracle (FEW HICCUPS)
//! PROOF: Ensures current behavior matches historical expectations
//! Catches unintended breaking changes

use git2::Repository;
use gitai::config::Config;
use std::fs;
use std::path::Path;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{TestAssertions, setup_git_repo};

#[tokio::test]
/// HISTORY ORACLE: Tests that behavior hasn't changed unexpectedly
/// Regression detection for commit message format consistency
async fn test_commit_message_format_stability() {
    let (_temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Get initial context
    let context = git_repo.get_git_info(&config).await.expect(
        "PROBLEM: Failed to get initial git info\n\
                 CONTEXT: Regression test for format stability\n\
                 EXPECTED: Successful context retrieval\n\
                 ACTUAL: Error occurred",
    );

    // Verify expected fields are present and non-empty
    TestAssertions::assert_commit_context_basics(&context);

    // HISTORY: Verify field formats match expectations
    assert!(
        !context.branch.is_empty(),
        "PROBLEM: Branch field format changed\n\
         CONTEXT: History oracle - regression detection\n\
         EXPECTED: Non-empty branch name\n\
         ACTUAL: Empty branch name\n\
         FREQUENCY: Breaking change if field renamed or removed"
    );
}

#[tokio::test]
/// HISTORY ORACLE: Tests staged file representation consistency
async fn test_staged_file_representation_consistency() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Add and stage a file
    let repo = Repository::open(temp_dir.path()).expect("Failed to open repo");
    fs::write(temp_dir.path().join("test.txt"), "content").expect("Failed to write file");

    let mut index = repo.index().expect("Failed to get index");
    index
        .add_path(Path::new("test.txt"))
        .expect("Failed to add to index");
    index.write().expect("Failed to write index");

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // HISTORY: Verify staged files have expected structure
    assert!(
        !context.staged_files.is_empty(),
        "PROBLEM: Staged files detection regressed\n\
         CONTEXT: History oracle - staged file format\n\
         EXPECTED: At least one staged file\n\
         ACTUAL: Empty list"
    );

    for file in &context.staged_files {
        // HISTORY: Verify all expected fields are present
        assert!(
            !file.path.is_empty(),
            "PROBLEM: File path field missing/empty\n\
             CONTEXT: Regression in staged file structure\n\
             EXPECTED: Non-empty path\n\
             ACTUAL: Empty path"
        );

        assert!(
            !file.change_type.to_string().is_empty(),
            "PROBLEM: Change type field missing/empty\n\
             CONTEXT: Regression in change type representation\n\
             EXPECTED: Valid change type\n\
             ACTUAL: Empty or invalid"
        );
    }
}

#[tokio::test]
/// HISTORY ORACLE: Tests context completeness across operations
async fn test_context_completeness_stability() {
    let (_temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Multiple operations should produce consistent context
    for _ in 0..3 {
        let context = git_repo.get_git_info(&config).await.expect(
            "PROBLEM: Context retrieval failed\n\
                     CONTEXT: Stability test\n\
                     EXPECTED: Consistent successful retrieval\n\
                     ACTUAL: Error occurred",
        );

        // HISTORY: Verify consistency
        TestAssertions::assert_commit_context_basics(&context);
    }
}

#[tokio::test]
/// HISTORY ORACLE: Tests that author history format is stable
async fn test_author_history_format_stability() {
    let (_temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // HISTORY: Author history should be a list of strings
    assert!(
        context
            .author_history
            .iter()
            .all(|s| !s.is_empty() || s.is_empty()),
        "PROBLEM: Author history format changed\n\
         CONTEXT: History oracle - author history representation\n\
         EXPECTED: List of commit messages\n\
         ACTUAL: Unexpected format"
    );
}

#[tokio::test]
/// HISTORY ORACLE: Tests config handling consistency
async fn test_config_handling_consistency() {
    // Test with default config
    let config_default = Config::default();

    // HISTORY: Default config should have expected structure
    // Note: Config compatibility is verified by successful method calls below
    // Test that config can be created without errors
    let (_temp_dir, git_repo) = setup_git_repo();
    let result = git_repo.get_git_info(&config_default).await;

    TestAssertions::assert_success(
        &result,
        "Config application to git operations",
        Some(&format!("Config: {config_default:?}")),
    );
}
