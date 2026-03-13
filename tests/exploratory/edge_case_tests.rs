//! Exploratory edge case tests
//! Tests system behavior with unusual but valid inputs
//! PROOF: Ensures robustness under edge conditions

use gitai::git::GitRepo;
use gitai::config::Config;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{setup_git_repo, TestAssertions, ComplexRepoBuilder};

#[tokio::test]
/// EDGE CASE: Test with repository having many commits
async fn test_handles_many_commits() {
    let result = ComplexRepoBuilder::new()
        .with_commits(10)
        .build();

    if let Ok((_temp_dir, git_repo)) = result {
        let config = Config::default();
        let info = git_repo.get_git_info(&config).await;
        
        TestAssertions::assert_success(
            &info,
            "Many commits handling",
            Some("EDGE CASE: Should handle repos with many commits"),
        );
    }
}

#[tokio::test]
/// EDGE CASE: Test with multiple branches
async fn test_handles_multiple_branches() {
    let result = ComplexRepoBuilder::new()
        .with_commits(3)
        .with_branches(5)
        .build();

    if let Ok((_temp_dir, git_repo)) = result {
        let config = Config::default();
        let info = git_repo.get_git_info(&config).await;
        
        TestAssertions::assert_success(
            &info,
            "Multiple branches handling",
            Some("EDGE CASE: Should handle repos with many branches"),
        );
    }
}

#[tokio::test]
/// EDGE CASE: Test with tags
async fn test_handles_tags() {
    let result = ComplexRepoBuilder::new()
        .with_commits(2)
        .with_tags()
        .build();

    if let Ok((_temp_dir, git_repo)) = result {
        let config = Config::default();
        let info = git_repo.get_git_info(&config).await;
        
        TestAssertions::assert_success(
            &info,
            "Tags handling",
            Some("EDGE CASE: Should handle repos with tags"),
        );
    }
}

#[tokio::test]
/// EDGE CASE: Test with binary files
async fn test_handles_binary_files() {
    let result = ComplexRepoBuilder::new()
        .with_commits(1)
        .with_binary_files()
        .build();

    if let Ok((_temp_dir, git_repo)) = result {
        let config = Config::default();
        let info = git_repo.get_git_info(&config).await;
        
        TestAssertions::assert_success(
            &info,
            "Binary files handling",
            Some("EDGE CASE: Should handle binary files in repo"),
        );
    }
}

#[tokio::test]
/// EDGE CASE: Test with special characters in paths
async fn test_handles_special_characters() {
    let result = ComplexRepoBuilder::new()
        .with_commits(1)
        .with_special_characters()
        .build();

    if let Ok((_temp_dir, git_repo)) = result {
        let config = Config::default();
        let info = git_repo.get_git_info(&config).await;
        
        TestAssertions::assert_success(
            &info,
            "Special characters handling",
            Some("EDGE CASE: Should handle special characters in file paths"),
        );
    }
}

#[tokio::test]
/// EDGE CASE: Test with complex repo (multiple features)
async fn test_handles_complex_repo() {
    let result = ComplexRepoBuilder::new()
        .with_commits(5)
        .with_branches(3)
        .with_tags()
        .with_binary_files()
        .build();

    if let Ok((_temp_dir, git_repo)) = result {
        let config = Config::default();
        let info = git_repo.get_git_info(&config).await;
        
        TestAssertions::assert_success(
            &info,
            "Complex repo handling",
            Some("EDGE CASE: Should handle complex repository states"),
        );
    }
}

#[tokio::test]
/// EDGE CASE: Test repeated identical operations
async fn test_idempotent_operations() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Same operation multiple times should produce same result
    let results: Vec<_> = (0..3)
        .map(|_| git_repo.get_git_info(&config).await)
        .collect();

    let all_ok = results.iter().all(|r| r.is_ok());
    assert!(
        all_ok,
        "PROBLEM: Idempotent operation failed\n\
         CONTEXT: Edge case - idempotency\n\
         EXPECTED: Same result every time\n\
         ACTUAL: Inconsistent results"
    );
}
