//! Fallback behavior documentation tests
//!
//! These tests document the CURRENT behavior of branch resolution.
//! Note: `get_git_info_for_branch_diff` uses `resolve_branch_strict` which does NOT fallback.
//!
//! Oracle: **C**laims - if you ask for X, you should get X or an error (not Y)

use git2::Repository;
use gitai::config::Config;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{GitTestHelper, setup_git_repo};

#[tokio::test]
/// FALLBACK: Document that `resolve_branch_strict` is used (no fallback)
/// Oracle: Claims - strict mode means strict, no fallback
async fn test_strict_mode_no_fallback() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create feature branch
    helper
        .create_branch("feature-strict")
        .expect("Failed to create feature");

    // Request nonexistent base - should fail (no fallback in strict mode)
    let config = Config::default();
    let result = git_repo.get_git_info_for_branch_diff(&config, "nonexistent", "feature-strict");

    // Should fail because resolve_branch_strict is used
    assert!(
        result.is_err(),
        "Should fail with nonexistent branch (no fallback)"
    );
}

#[tokio::test]
/// FALLBACK: Verify fallback branches exist in repo
/// Oracle: History - these are the branches that WOULD be used if fallback was enabled
async fn test_fallback_branches_exist() {
    let (temp_dir, _git_repo) = setup_git_repo();
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");

    // Get all branches
    let branches: Vec<String> = repo
        .branches(None)
        .expect("Failed to get branches")
        .filter_map(std::result::Result::ok)
        .filter_map(|(b, _)| b.name().ok().flatten().map(String::from))
        .collect();

    // Verify main exists (the first fallback choice)
    assert!(
        branches.contains(&"main".to_string()),
        "main branch should exist for fallback"
    );
}

#[tokio::test]
/// FALLBACK: Deterministic behavior - same input gives same error
/// Oracle: History - behavior should be predictable
async fn test_deterministic_error() {
    let (_temp_dir, git_repo) = setup_git_repo();

    let config = Config::default();

    // Multiple calls with same input should give same result
    let results: Vec<_> = (0..3)
        .map(|_| git_repo.get_git_info_for_branch_diff(&config, "nonexistent", "also-nonexistent"))
        .collect();

    // All should fail consistently
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_err(), "Call {i} should fail consistently");
    }
}

#[tokio::test]
/// FALLBACK: When explicit branch exists, use it (no fallback needed)
/// Oracle: Claims - if you ask for X and it exists, you get X
async fn test_explicit_branch_used() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create a branch named "develop"
    helper
        .create_branch("develop")
        .expect("Failed to create develop");

    // Request "develop" explicitly - should use it
    let config = Config::default();
    let context = git_repo
        .get_git_info_for_branch_diff(&config, "main", "develop")
        .expect("Failed to get branch diff");

    assert_eq!(context.branch, "main -> develop");
}

#[tokio::test]
/// FALLBACK: Error mentions what was requested
/// Oracle: User Expectations - user should know what branch failed
async fn test_error_mentions_requested() {
    let (_temp_dir, git_repo) = setup_git_repo();

    let config = Config::default();
    let result = git_repo.get_git_info_for_branch_diff(&config, "my-branch", "target");

    assert!(result.is_err(), "Should fail");

    let err = result.expect_err("Should fail with nonexistent branch");
    assert!(
        err.to_string().contains("my-branch") || err.to_string().contains("target"),
        "Error should mention requested branch"
    );
}

#[tokio::test]
/// FALLBACK: Future enhancement - if `resolve_branch` was used instead
/// Oracle: History - this documents what WOULD happen if fallback was enabled
async fn test_fallback_documentation() {
    // This test documents the INTENDED fallback behavior for future implementation
    // Currently get_git_info_for_branch_diff uses resolve_branch_strict
    // If changed to use resolve_branch, the fallback order would be:
    // main -> master -> develop -> development

    let fallback_order = ["main", "master", "develop", "development"];

    // Document the intended order
    assert_eq!(fallback_order.len(), 4, "Fallback should have 4 branches");
    assert_eq!(fallback_order[0], "main", "main should be first fallback");
}
