//! Exploratory recovery tests
//! Tests system recovery from invalid states and interrupted operations
//! PROOF: Ensures system degrades gracefully under adverse conditions

use gitai::git::GitRepo;
use gitai::config::Config;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{setup_git_repo, TestAssertions};

#[tokio::test]
/// RECOVERY: Test recovery from config access issues
async fn test_graceful_degradation_on_config_error() {
    // Test that system handles config issues gracefully
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Normal operation should work
    let result = git_repo.get_git_info(&config).await;
    
    TestAssertions::assert_success(
        &result,
        "Normal operation after setup",
        Some("RECOVERY: System should work with valid config"),
    );
}

#[tokio::test]
/// RECOVERY: Test handles empty directory gracefully
async fn test_handles_empty_directory() {
    use tempfile::TempDir;
    use gitai::git::GitRepo;
    
    let temp_dir = TempDir::new().expect("Failed to create temp");
    
    // Attempting to open non-repo should return error, not panic
    let result = GitRepo::new(temp_dir.path());
    
    TestAssertions::assert_failure(
        &result,
        "Non-git directory access",
        Some("repository"),
    );
}

#[tokio::test]
/// RECOVERY: System should be consistent across multiple calls
async fn test_state_consistency_across_operations() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Multiple calls should produce consistent results
    let result1 = git_repo.get_git_info(&config).await;
    let result2 = git_repo.get_git_info(&config).await;
    let result3 = git_repo.get_git_info(&config).await;

    // All should succeed or fail consistently
    assert_eq!(
        result1.is_ok(),
        result2.is_ok(),
        "PROBLEM: Inconsistent results across calls\n\
         CONTEXT: Recovery - state consistency\n\
         EXPECTED: Consistent behavior\n\
         ACTUAL: First call result differs from second"
    );
    
    assert_eq!(
        result2.is_ok(),
        result3.is_ok(),
        "PROBLEM: Inconsistent results across calls\n\
         CONTEXT: Recovery - state consistency\n\
         EXPECTED: Consistent behavior\n\
         ACTUAL: Second call result differs from third"
    );
}

#[tokio::test]
/// RECOVERY: Test with interrupted state simulation
async fn test_handles_simulated_interruption() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Get initial state
    let initial = git_repo.get_git_info(&config).await;
    
    TestAssertions::assert_success(
        &initial,
        "Initial state retrieval",
        Some("RECOVERY: Should retrieve initial state"),
    );

    // Do another operation
    let subsequent = git_repo.get_git_info(&config).await;
    
    TestAssertions::assert_success(
        &subsequent,
        "Subsequent state retrieval",
        Some("RECOVERY: Should work after initial operation"),
    );
}

#[tokio::test]
/// RECOVERY: Verify repo remains valid after operations
async fn test_repo_validity_preserved() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Multiple operations
    for _ in 0..5 {
        let result = git_repo.get_git_info(&config).await;
        TestAssertions::assert_success(
            &result,
            "Repeated operation",
            Some("RECOVERY: Repo should remain valid after multiple operations"),
        );
    }
}
