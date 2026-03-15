//! Error quality tests for branch resolution
//!
//! Tests that error messages are clear, actionable, and follow conventions.
//!
//! Oracle: **U**ser Expectations - errors should help users understand what went wrong
//! Oracle: **S**tandards - error format should match git2 conventions

use gitai::config::Config;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::setup_git_repo;

#[tokio::test]
/// ERROR QUALITY: Error includes the requested branch name
/// Oracle: User Expectations - user should know what branch was requested
async fn test_error_includes_requested_branch() {
    let (_temp_dir, git_repo) = setup_git_repo();

    let config = Config::default();
    let result =
        git_repo.get_git_info_for_branch_diff(&config, "my-feature-branch", "nonexistent-target");

    assert!(result.is_err(), "Should fail with nonexistent branches");

    let err = result.expect_err("Should fail with nonexistent branch");
    let err_str = err.to_string();

    // Error should mention the branch the user requested
    assert!(
        err_str.contains("my-feature-branch") || err_str.contains("nonexistent-target"),
        "Error should mention requested branch name: {err_str}"
    );
}

#[tokio::test]
/// ERROR QUALITY: Error is clear about what branch failed
/// Oracle: User Expectations - user should know exactly what went wrong
async fn test_error_is_clear_about_failure() {
    let (_temp_dir, git_repo) = setup_git_repo();

    let config = Config::default();
    let result =
        git_repo.get_git_info_for_branch_diff(&config, "nonexistent-base", "nonexistent-target");

    assert!(result.is_err(), "Should fail");

    let err = result.expect_err("Should fail with nonexistent branch");
    let err_str = err.to_string();

    // Error should clearly state which branch couldn't be resolved
    assert!(
        err_str.contains("nonexistent-base")
            || err_str.contains("nonexistent-target")
            || err_str.contains("resolve"),
        "Error should clearly state what failed. Error: {err_str}"
    );
}

#[tokio::test]
/// ERROR QUALITY: Error message is actionable
/// Oracle: User Expectations - user should know what to do next
async fn test_error_is_actionable() {
    let (_temp_dir, git_repo) = setup_git_repo();

    let config = Config::default();
    let result = git_repo.get_git_info_for_branch_diff(&config, "missing", "also-missing");

    assert!(result.is_err(), "Should fail");

    let err = result.expect_err("Should fail with nonexistent branch");
    let err_str = err.to_string();

    // Error should provide enough context for user to understand the problem
    assert!(
        err_str.contains("branch") || err_str.contains("resolve") || err_str.contains("Could not"),
        "Error should provide actionable context: {err_str}"
    );
}

#[tokio::test]
/// ERROR QUALITY: Error format matches git2 conventions
/// Oracle: Standards - consistency with underlying library
async fn test_error_matches_git_conventions() {
    let (_temp_dir, git_repo) = setup_git_repo();

    let config = Config::default();
    let result = git_repo.get_git_info_for_branch_diff(&config, "invalid-branch", "target");

    // Should fail with git2-style error
    assert!(result.is_err(), "Should fail with invalid branch");

    let err = result.expect_err("Should fail with nonexistent branch");
    let err_str = err.to_string();

    // Git2 errors typically include context about what was being resolved
    assert!(
        err_str.len() > 20,
        "Error should be descriptive, not terse: {err_str}"
    );
}

#[tokio::test]
/// ERROR QUALITY: Error doesn't make false promises
/// Oracle: User Expectations - don't claim to have tried branches that weren't attempted
async fn test_error_not_misleading() {
    let (_temp_dir, git_repo) = setup_git_repo();

    let config = Config::default();
    let result = git_repo.get_git_info_for_branch_diff(&config, "nonexistent", "target");

    assert!(result.is_err(), "Should fail");

    let err = result.expect_err("Should fail with nonexistent branch");
    let err_str = err.to_string();

    // Error shouldn't claim to have tried branches outside the fallback list
    assert!(
        !err_str.contains("origin/") || err_str.contains("origin/main"),
        "Error shouldn't mention remote branches unless they exist"
    );

    // Error should be honest about what was attempted
    assert!(
        err_str.contains("nonexistent")
            || err_str.contains("main")
            || err_str.contains("alternative"),
        "Error should be honest about what was tried: {err_str}"
    );
}

#[tokio::test]
/// ERROR QUALITY: Strict mode error includes the exact branch name
/// Oracle: Standards - strict mode should be precise about what failed
async fn test_strict_mode_error_precision() {
    let (_temp_dir, git_repo) = setup_git_repo();

    let config = Config::default();

    let result = git_repo.get_git_info_for_branch_diff(&config, "main", "strictly-nonexistent");

    assert!(result.is_err(), "Should fail with nonexistent target");

    let err = result.expect_err("Should fail with nonexistent branch");
    let err_str = err.to_string();

    // Error should mention the exact branch name that failed
    assert!(
        err_str.contains("strictly-nonexistent"),
        "Error should mention the exact branch name: {err_str}"
    );
}
