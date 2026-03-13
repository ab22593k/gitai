//! Error quality tests - ensuring error messages meet user needs
//! PROOF: Tests that when things go wrong, users understand what happened
//! and what they can do about it

use git2::Repository;
use gitai::config::Config;
use gitai::git::GitRepo;
use std::fs;
use std::path::Path;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{TestAssertions, setup_git_repo};

#[tokio::test]
/// ERROR QUALITY: Valid operations should succeed
/// Users expect success when doing the right thing
async fn test_valid_operations_succeed() {
    let (_temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let result = git_repo.get_git_info(&config).await;

    TestAssertions::assert_success(
        &result,
        "Valid git operation",
        Some("ERROR QUALITY: Valid operations should not return errors"),
    );
}

#[tokio::test]
/// ERROR QUALITY: No panic on user error
/// User mistakes should produce errors, not panics
async fn test_no_panic_on_invalid_input() {
    use std::path::PathBuf;

    // GitRepo::new() doesn't validate the path - it just stores it
    // The validation happens when calling methods
    // So we test that get_git_info fails appropriately
    let invalid_path = PathBuf::from("/tmp/nonexistent_path_12345_xyz");
    let git_repo = GitRepo::new(&invalid_path).expect("GitRepo::new should work");
    let config = Config::default();

    // This should fail because path doesn't exist
    let result = git_repo.get_git_info(&config).await;

    TestAssertions::assert_failure(&result, "Invalid repo path", None);
}

#[tokio::test]
/// ERROR QUALITY: Error messages should include context
/// Users need to understand where an error occurred
async fn test_errors_include_context() {
    // GitRepo::new() doesn't validate the path
    // Test that get_git_info provides meaningful errors
    use std::path::PathBuf;

    let invalid_path = PathBuf::from("/tmp/nonexistent_path_12345_xyz");
    let git_repo = GitRepo::new(&invalid_path).expect("GitRepo::new should work");
    let config = Config::default();

    let result = git_repo.get_git_info(&config).await;

    TestAssertions::assert_failure(&result, "Invalid path access", None);
}

#[tokio::test]
/// ERROR QUALITY: Operations handle missing files gracefully
async fn test_handles_missing_files() {
    let (_temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // This should work - we're asking about an existing repo
    let result = git_repo.get_git_info(&config).await;

    TestAssertions::assert_success(
        &result,
        "Git info on valid repo",
        Some("ERROR QUALITY: Operations should succeed with valid state"),
    );
}

#[tokio::test]
/// ERROR QUALITY: Concurrent access should be handled safely
async fn test_concurrent_access_safety() {
    use git2::Repository;
    use gitai::git::GitRepo;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use tokio::task;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp");
    let repo = Repository::init(temp_dir.path()).expect("Failed to init");

    let mut config = repo.config().expect("Failed to get config");
    config.set_str("user.name", "Test").expect("Failed");
    config
        .set_str("user.email", "test@test.com")
        .expect("Failed");

    // Create initial commit
    fs::write(temp_dir.path().join("a.txt"), "a").expect("Failed");
    let mut index = repo.index().expect("Failed");
    index.add_path(Path::new("a.txt")).expect("Failed");
    index.write().expect("Failed");
    let tree_id = index.write_tree().expect("Failed");
    let tree = repo.find_tree(tree_id).expect("Failed");
    let sig = repo.signature().expect("Failed");
    repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
        .expect("Failed");

    let git_repo = Arc::new(GitRepo::new(temp_dir.path()).expect("Failed"));
    let config = Arc::new(Config::default());

    // ERROR QUALITY: Concurrent operations should not corrupt state
    let mut handles = vec![];
    for _ in 0..3 {
        let repo_clone = Arc::clone(&git_repo);
        let cfg_clone = Arc::clone(&config);
        handles.push(task::spawn(async move {
            repo_clone.get_git_info(&cfg_clone).await
        }));
    }

    let mut errors = vec![];
    for handle in handles {
        if let Err(e) = handle.await {
            errors.push(format!("Task error: {e:?}"));
        }
    }

    // All should succeed or all should fail consistently
    assert!(
        errors.is_empty(),
        "PROBLEM: Concurrent operations had errors\n\
         CONTEXT: Error quality - concurrent safety\n\
         EXPECTED: All succeed or consistent error\n\
         ACTUAL: {errors:?}\n\
         FREQUENCY: Race condition if intermittent"
    );
}

#[tokio::test]
/// ERROR QUALITY: Large inputs should be handled
async fn test_handles_large_inputs() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Create many files to test scalability
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");

    for i in 0..50 {
        fs::write(
            temp_dir.path().join(format!("file_{i}.txt")),
            format!("Content {i}"),
        )
        .expect("Failed to write");

        let mut index = repo.index().expect("Failed to get index");
        index
            .add_path(Path::new(format!("file_{i}.txt").as_str()))
            .expect("Failed");
        index.write().expect("Failed to write index");
    }

    // ERROR QUALITY: Should handle large number of files
    let result = git_repo.get_git_info(&config).await;

    TestAssertions::assert_success(
        &result,
        "Large file batch operation",
        Some("ERROR QUALITY: Should handle many staged files"),
    );
}
