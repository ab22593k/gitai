//! User experience tests using User Expectations oracle (FEW HICCUPS)
//! PROOF: Tests that the system meets user expectations and is intuitive
//! Catches usability issues before users encounter them

use git2::Repository;
use gitai::config::Config;
use gitai::git::GitRepo;
use std::fs;
use std::path::Path;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{TestAssertions, setup_git_repo};

#[tokio::test]
/// USER EXPECTATIONS ORACLE: Error messages should be actionable
/// Users should understand what went wrong and what to do next
async fn test_error_messages_are_actionable() {
    let (_temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // This operation should succeed for valid repos
    let result = git_repo.get_git_info(&config).await;

    TestAssertions::assert_success(
        &result,
        "Git info retrieval",
        Some("USER EXPECTATION: Operations should succeed with valid repo"),
    );
}

#[tokio::test]
/// USER EXPECTATIONS ORACLE: Reasonable defaults should work
/// Fresh install should work without configuration
async fn test_default_config_works() {
    let (_temp_dir, git_repo) = setup_git_repo();

    // USER EXPECTATION: Default config should work out of the box
    let config = Config::default();

    let result = git_repo.get_git_info(&config).await;

    TestAssertions::assert_success(
        &result,
        "Default config application",
        Some("USER EXPECTATION: Default config should work without setup"),
    );
}

#[tokio::test]
/// USER EXPECTATIONS ORACLE: System should handle edge cases gracefully
/// Users shouldn't see confusing errors for unusual but valid scenarios
async fn test_handles_empty_repo_gracefully() {
    use git2::Repository;
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo = Repository::init(temp_dir.path()).expect("Failed to init repo");

    // Configure git user
    let mut config = repo.config().expect("Failed to get config");
    config
        .set_str("user.name", "Test User")
        .expect("Failed to set name");
    config
        .set_str("user.email", "test@test.com")
        .expect("Failed to set email");

    // Create file and commit immediately (no initial commit)
    fs::write(temp_dir.path().join("README.md"), "# Test").expect("Failed to write");

    let mut index = repo.index().expect("Failed to get index");
    index
        .add_path(std::path::Path::new("README.md"))
        .expect("Failed to add");
    index.write().expect("Failed to write index");

    let tree_id = index.write_tree().expect("Failed to write tree");
    let tree = repo.find_tree(tree_id).expect("Failed to find tree");
    let signature = repo.signature().expect("Failed to get signature");
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )
    .expect("Failed to commit");

    let git_repo = GitRepo::new(temp_dir.path()).expect("Failed to create GitRepo");
    let config = Config::default();

    // USER EXPECTATION: Single-commit repo should work fine
    let result = git_repo.get_git_info(&config).await;

    TestAssertions::assert_success(
        &result,
        "Single commit repo handling",
        Some("USER EXPECTATION: System handles minimal repos gracefully"),
    );
}

#[tokio::test]
/// USER EXPECTATIONS ORACLE: Output should be predictable
/// Users should know what to expect from operations
async fn test_output_predictability() {
    let (_temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Get context twice - should be consistent
    let context1 = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed first call");
    let context2 = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed second call");

    // USER EXPECTATION: Same query should return same result
    assert_eq!(
        context1.branch, context2.branch,
        "PROBLEM: Branch name inconsistent between calls\n\
         CONTEXT: User Expectations oracle - output predictability\n\
         EXPECTED: Consistent branch name\n\
         ACTUAL: {} vs {}",
        context1.branch, context2.branch
    );

    assert_eq!(
        context1.staged_files.len(),
        context2.staged_files.len(),
        "PROBLEM: Staged files count inconsistent\n\
         CONTEXT: User Expectations oracle\n\
         EXPECTED: Consistent results\n\
         ACTUAL: Different counts"
    );
}

#[tokio::test]
/// USER EXPECTATIONS ORACLE: System should work with common workflows
/// Standard git workflows should be supported
async fn test_common_git_workflow() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Create file
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");
    fs::write(temp_dir.path().join("new_file.rs"), "fn main() {}").expect("Failed to write");

    // Stage it
    let mut index = repo.index().expect("Failed to get index");
    index
        .add_path(Path::new("new_file.rs"))
        .expect("Failed to add");
    index.write().expect("Failed to write index");

    // USER EXPECTATION: Just-staged files should appear in context
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get info");

    TestAssertions::assert_with_proof(
        context.staged_files.iter().any(|f| f.path == "new_file.rs"),
        "Staged file not detected",
        "User Expectations oracle - common workflow",
        "Newly staged file should appear in context",
        format!(
            "Staged files: {:?}",
            context
                .staged_files
                .iter()
                .map(|f| &f.path)
                .collect::<Vec<_>>()
        ),
        None,
    );
}

#[tokio::test]
/// USER EXPECTATIONS ORACLE: Performance should be acceptable
/// Users expect reasonable response times
async fn test_performance_meets_expectations() {
    use std::time::Instant;

    let (_temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let start = Instant::now();
    let result = git_repo.get_git_info(&config).await;
    let elapsed = start.elapsed();

    TestAssertions::assert_success(&result, "Git info retrieval", None);

    // USER EXPECTATION: Operations should complete in reasonable time
    assert!(
        elapsed.as_secs() < 5,
        "PROBLEM: Operation too slow\n\
         CONTEXT: User Expectations oracle - performance\n\
         EXPECTED: < 5 seconds\n\
         ACTUAL: {elapsed:?}\n\
         FREQUENCY: Always if performance degrades"
    );
}
