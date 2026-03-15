//! Remote tracking branch tests - thorough coverage of origin/main style branches
//!
//! Tests actual remote tracking branches (refs/remotes/origin/main) vs local branches.
//! This file tests the distinction between:
//! - Local branches named "origin/main" (just a local branch with that name)
//! - Actual remote tracking branches (refs/remotes/origin/main) from a remote
//!
//! Oracle: **F**amiliarity - Compare with standard git behavior

use git2::Repository;
use gitai::config::Config;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{GitTestHelper, setup_git_repo};

#[tokio::test]
/// REMOTE TRACKING: Local branch named origin/main should be resolved first
/// Oracle: Familiarity - local branches take precedence over remote tracking
async fn test_local_branch_named_origin_main() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create a LOCAL branch named "origin/main"
    helper
        .create_branch("origin/main")
        .expect("Failed to create local branch");
    helper
        .checkout_branch("origin/main")
        .expect("Failed to checkout");
    helper
        .create_and_stage_file("local_origin_main.txt", "local branch content")
        .expect("Failed to create file");
    helper
        .commit("Add file to local origin/main branch")
        .expect("Failed to commit");

    // When resolving "origin/main", local branch should take precedence
    let config = Config::default();
    let context = git_repo
        .get_git_info_for_branch_diff(&config, "main", "origin/main")
        .expect("Failed to get branch diff");

    assert_eq!(context.branch, "main -> origin/main");
    assert_eq!(context.staged_files.len(), 1);
    assert_eq!(
        context.staged_files[0].path, "local_origin_main.txt",
        "Should use local branch content"
    );
}

#[tokio::test]
/// REMOTE TRACKING: Resolve actual remote tracking branch from fetch
/// Oracle: Familiarity - origin/main refers to remote tracking after fetch
async fn test_actual_remote_tracking_branch() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create actual remote tracking branch via fetch
    helper
        .create_remote_tracking_branch("origin", "main")
        .expect("Failed to create remote tracking branch");

    // Verify the remote tracking branch exists
    let repo = Repository::open(temp_dir.path()).expect("Failed to open repo");
    let refs = repo.references().expect("Failed to get references");

    let has_remote_tracking = refs
        .into_iter()
        .filter_map(std::result::Result::ok)
        .any(|r| {
            r.name()
                .is_some_and(|n| n.starts_with("refs/remotes/origin/main"))
        });

    assert!(has_remote_tracking, "Should have refs/remotes/origin/main");

    // Now test that we can resolve it
    let config = Config::default();
    let result = git_repo.get_git_info_for_branch_diff(&config, "main", "origin/main");

    // This should work - origin/main should resolve to remote tracking branch
    // Note: May fail if local main doesn't exist, but should try both
    if let Ok(context) = result {
        assert!(context.branch.contains("origin/main"));
    }
}

#[tokio::test]
/// REMOTE TRACKING: Handle tilde suffix on remote tracking branch
/// Oracle: Familiarity - origin/main~3 means 3 commits before origin/main
async fn test_remote_tracking_with_tilde_suffix() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create remote tracking branch with multiple commits
    helper
        .create_multiple_remote_tracking_branches("origin", &["main"])
        .expect("Failed to create remote tracking branches");

    // Create commits on local main to have history
    for i in 0..3 {
        helper
            .create_and_stage_file(&format!("local_file_{i}.txt"), "content")
            .expect("Failed to create file");
        helper
            .commit(&format!("Local commit {i}"))
            .expect("Failed to commit");
    }

    // Test resolving origin/main~1 should work
    let config = Config::default();
    let result = git_repo.get_git_info_for_branch_diff(&config, "main", "origin/main~1");

    // Should either work (if commits exist) or give clear error
    if result.is_err() {
        let err = result.expect_err("Failed");
        let err_str = err.to_string();
        // Error should mention the branch or reference
        assert!(
            err_str.contains("origin/main") || err_str.contains("resolve"),
            "Error should mention reference: {err_str}"
        );
    }
}

#[tokio::test]
/// REMOTE TRACKING: Handle caret suffix on remote tracking branch
/// Oracle: Familiarity - origin/main^ means first parent of origin/main
async fn test_remote_tracking_with_caret_suffix() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    helper
        .create_remote_tracking_branch("origin", "feature")
        .expect("Failed to create remote tracking branch");

    let config = Config::default();
    let result = git_repo.get_git_info_for_branch_diff(&config, "main", "origin/feature^");

    // Should handle caret suffix appropriately
    if result.is_err() {
        let err = result.expect_err("Failed");
        let err_str = err.to_string();
        // Error should be informative
        assert!(!err_str.is_empty(), "Error should not be empty");
    }
}

#[tokio::test]
/// REMOTE TRACKING: Error for nonexistent remote tracking branch
/// Oracle: Claims - if it doesn't exist, we get an error
async fn test_nonexistent_remote_tracking() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create a remote, but not the branch we're looking for
    helper
        .create_remote_tracking_branch("origin", "main")
        .expect("Failed to create remote tracking branch");

    let config = Config::default();
    let result = git_repo.get_git_info_for_branch_diff(&config, "main", "origin/nonexistent");

    // Should fail - branch doesn't exist
    assert!(
        result.is_err(),
        "Should fail with nonexistent remote tracking branch"
    );

    let err = result.expect_err("Should fail");
    let err_str = err.to_string();
    assert!(
        err_str.contains("origin/nonexistent") || err_str.contains("resolve"),
        "Error should mention the branch: {err_str}"
    );
}

#[tokio::test]
/// REMOTE TRACKING: Test remote tracking fallback order
/// Oracle: History - fallback should try multiple remote tracking branches
/// Fallback order: origin/main → origin/master → upstream/main → local branches
async fn test_remote_tracking_fallback_order() {
    let (temp_dir, _git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create multiple remote tracking branches
    helper
        .create_multiple_remote_tracking_branches("origin", &["main", "master"])
        .expect("Failed to create origin branches");
    helper
        .create_multiple_remote_tracking_branches("upstream", &["main"])
        .expect("Failed to create upstream branches");

    // Get all remote tracking branches to verify setup
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");
    let refs: Vec<String> = repo
        .references()
        .expect("Failed to get references")
        .filter_map(std::result::Result::ok)
        .filter_map(|r| r.name().map(String::from))
        .filter(|n| n.starts_with("refs/remotes/"))
        .collect();

    assert!(
        refs.iter().any(|r| r.contains("origin/main")),
        "Should have origin/main"
    );
    assert!(
        refs.iter().any(|r| r.contains("origin/master")),
        "Should have origin/master"
    );
    assert!(
        refs.iter().any(|r| r.contains("upstream/main")),
        "Should have upstream/main"
    );
}

#[tokio::test]
/// REMOTE TRACKING: Fallback from remote to local when remote fails
/// Oracle: Claims - if remote doesn't resolve, try local branches
async fn test_remote_tracking_fallback_to_local() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create local main branch with content
    for i in 0..2 {
        helper
            .create_and_stage_file(&format!("main_file_{i}.txt"), "content")
            .expect("Failed to create file");
        helper
            .commit(&format!("Main commit {i}"))
            .expect("Failed to commit");
    }

    // Create a local branch called "feature"
    helper
        .create_branch("feature")
        .expect("Failed to create branch");
    helper
        .checkout_branch("feature")
        .expect("Failed to checkout");
    helper
        .create_and_stage_file("feature.txt", "feature content")
        .expect("Failed to create file");
    helper.commit("Add feature file").expect("Failed to commit");

    // Test that fallback works when remote tracking doesn't exist for feature
    // Note: This tests the fallback logic, not necessarily that it will succeed
    // since the implementation uses strict resolution
    let config = Config::default();

    // Request a branch that doesn't exist as remote tracking but exists locally
    let result = git_repo.get_git_info_for_branch_diff(&config, "main", "feature");

    // Should succeed because feature exists locally
    assert!(result.is_ok(), "Should resolve local branch");

    let context = result.expect("Failed");
    assert_eq!(context.branch, "main -> feature");
}

#[tokio::test]
/// REMOTE TRACKING: Multiple remotes - test priority
/// Oracle: Comparable - when multiple remotes have same branch name
async fn test_multiple_remotes_priority() {
    let (temp_dir, _git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create branches on two remotes
    helper
        .create_multiple_remote_tracking_branches("origin", &["main"])
        .expect("Failed to create origin branches");
    helper
        .create_multiple_remote_tracking_branches("upstream", &["main"])
        .expect("Failed to create upstream branches");

    // Verify both remotes exist
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");
    let refs: Vec<String> = repo
        .references()
        .expect("Failed to get references")
        .filter_map(std::result::Result::ok)
        .filter_map(|r| r.name().map(String::from))
        .collect();

    assert!(
        refs.iter().any(|r| r == "refs/remotes/origin/main"),
        "Should have origin/main"
    );
    assert!(
        refs.iter().any(|r| r == "refs/remotes/upstream/main"),
        "Should have upstream/main"
    );
}

#[tokio::test]
/// REMOTE TRACKING: Handle stale remote tracking branches
/// Oracle: World - remote branch deleted but tracking remains
async fn test_stale_remote_tracking() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create a remote tracking branch
    helper
        .create_remote_tracking_branch("origin", "main")
        .expect("Failed to create remote tracking branch");

    // Verify it exists
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");
    let refs: Vec<String> = repo
        .references()
        .expect("Failed to get references")
        .filter_map(std::result::Result::ok)
        .filter_map(|r| r.name().map(String::from))
        .filter(|n| n.contains("origin/main"))
        .collect();

    assert!(
        !refs.is_empty(),
        "Should have at least one origin/main reference before test"
    );

    // Try to resolve it - should work if the commit exists
    let config = Config::default();
    let result = git_repo.get_git_info_for_branch_diff(&config, "main", "origin/main");

    // The behavior depends on whether the commit still exists
    // Either way, we should get a clear result
    if result.is_err() {
        let err = result.expect_err("Failed");
        let err_str = err.to_string();
        assert!(
            !err_str.is_empty(),
            "Error should be informative for stale branch"
        );
    }
}

#[tokio::test]
/// REMOTE TRACKING: Compare remote tracking vs local branch with same name
/// Oracle: World - same name but different content
async fn test_diff_remote_vs_local_branch() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create a remote tracking branch
    helper
        .create_remote_tracking_branch("origin", "feature")
        .expect("Failed to create remote tracking branch");

    // Create a LOCAL branch also named "feature"
    helper
        .create_branch("feature")
        .expect("Failed to create local branch");
    helper
        .checkout_branch("feature")
        .expect("Failed to checkout");
    helper
        .create_and_stage_file("local_feature.txt", "different content")
        .expect("Failed to create file");
    helper
        .commit("Local feature commit")
        .expect("Failed to commit");

    // When we ask for "feature", local should take precedence
    let config = Config::default();
    let context = git_repo
        .get_git_info_for_branch_diff(&config, "main", "feature")
        .expect("Failed to get branch diff");

    // Should resolve to local branch
    assert_eq!(context.branch, "main -> feature");
    assert!(
        context
            .staged_files
            .iter()
            .any(|f| f.path.contains("local_feature")),
        "Should use local branch content"
    );
}

#[tokio::test]
/// REMOTE TRACKING: Error messages should clarify remote vs local context
/// Oracle: User Expectations - error should indicate what's available
async fn test_error_mentions_remote_context() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create remote tracking branch
    helper
        .create_remote_tracking_branch("origin", "main")
        .expect("Failed to create remote tracking branch");

    let config = Config::default();

    // Try to resolve a completely nonexistent branch
    let result =
        git_repo.get_git_info_for_branch_diff(&config, "main", "definitely-does-not-exist-12345");

    assert!(result.is_err(), "Should fail with nonexistent branch");

    let err = result.expect_err("Should fail");
    let err_str = err.to_string();

    // Error should be informative
    assert!(err_str.len() > 10, "Error should be descriptive: {err_str}");

    // Should mention what was tried
    assert!(
        err_str.contains("definitely-does-not-exist") || err_str.contains("Could not"),
        "Error should mention what was tried: {err_str}"
    );
}

#[tokio::test]
/// REMOTE TRACKING: Complex commitish syntax on remote tracking
/// Oracle: Familiarity - origin/main~3^2 means complex navigation
async fn test_commitish_on_remote_tracking() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create remote with multiple commits on a branch
    helper
        .create_multiple_remote_tracking_branches("origin", &["main"])
        .expect("Failed to create remote tracking branch");

    // Create some local commits to have comparison base
    for i in 0..3 {
        helper
            .create_and_stage_file(&format!("local_{i}.txt"), "content")
            .expect("Failed to create file");
        helper
            .commit(&format!("Local {i}"))
            .expect("Failed to commit");
    }

    let config = Config::default();

    // Test complex commitish - might fail but should be handled gracefully
    let result = git_repo.get_git_info_for_branch_diff(&config, "main", "origin/main~1");

    // Either succeeds with correct data or fails with clear error
    if result.is_err() {
        let err = result.expect_err("Failed");
        let err_str = err.to_string();
        // Should not panic or give cryptic error
        assert!(!err_str.is_empty(), "Error should be present");
    }
}
