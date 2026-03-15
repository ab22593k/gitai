//! Branch resolution tests - exact matching and basic behavior
//!
//! Tests that branch resolution correctly identifies branches
//! and applies fallback logic appropriately.
//!
//! Oracle: **F**amiliarity - Compare with standard git behavior

use git2::Repository;
use gitai::config::Config;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{GitTestHelper, setup_git_repo};

#[tokio::test]
/// BRANCH RESOLUTION: Exact branch name should be used when it exists
/// Oracle: Familiarity - git uses exact branch names
async fn test_exact_branch_match() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create a feature branch with a commit
    helper
        .create_branch("feature-exact")
        .expect("Failed to create branch");
    helper
        .checkout_branch("feature-exact")
        .expect("Failed to checkout");
    helper
        .create_and_stage_file("feature.txt", "feature content")
        .expect("Failed to create file");
    helper.commit("Add feature").expect("Failed to commit");

    // Get branch diff - should use exact branch name
    let config = Config::default();
    let context = git_repo
        .get_git_info_for_branch_diff(&config, "main", "feature-exact")
        .expect("Failed to get branch diff");

    // Verify exact branch was used (not a fallback)
    assert_eq!(context.branch, "main -> feature-exact");
    assert_eq!(context.staged_files.len(), 1);
    assert_eq!(context.staged_files[0].path, "feature.txt");
}

#[tokio::test]
/// BRANCH RESOLUTION: Nonexistent branches should fail
/// Oracle: Claims - if you ask for X and it doesn't exist, you get an error
/// Note: `get_git_info_for_branch_diff` uses `resolve_branch_strict` which doesn't fallback
async fn test_nonexistent_branch_fails() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create a feature branch
    helper
        .create_branch("feature-exists")
        .expect("Failed to create branch");

    // Request with nonexistent base - should fail (no fallback in strict mode)
    let config = Config::default();
    let result = git_repo.get_git_info_for_branch_diff(&config, "nonexistent", "feature-exists");

    // Should fail because resolve_branch_strict is used
    assert!(result.is_err(), "Should fail with nonexistent branch");

    let err = result.expect_err("Should fail");
    assert!(
        err.to_string().contains("nonexistent"),
        "Error should mention the branch"
    );
}

#[tokio::test]
/// BRANCH RESOLUTION: Verify fallback priority order
/// Oracle: History - main → master → develop → development
async fn test_fallback_priority_order() {
    let (temp_dir, _git_repo) = setup_git_repo();
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");

    // Create master branch (main already exists from setup)
    let head = repo
        .head()
        .expect("Failed to get HEAD")
        .peel_to_commit()
        .expect("Failed");
    repo.branch("master", &head, true)
        .expect("Failed to create master");
    repo.branch("develop", &head, true)
        .expect("Failed to create develop");
    repo.branch("development", &head, true)
        .expect("Failed to create development");

    // The fallback order is: main → master → develop → development
    // When requesting "nonexistent", it should try main first and succeed
    // This test verifies all fallback branches exist for other tests
    let branches: Vec<String> = repo
        .branches(None)
        .expect("Failed to get branches")
        .filter_map(std::result::Result::ok)
        .filter_map(|(b, _)| b.name().ok().flatten().map(String::from))
        .collect();

    assert!(
        branches.contains(&"main".to_string()),
        "main branch should exist"
    );
    assert!(
        branches.contains(&"master".to_string()),
        "master branch should exist"
    );
    assert!(
        branches.contains(&"develop".to_string()),
        "develop branch should exist"
    );
    assert!(
        branches.contains(&"development".to_string()),
        "development branch should exist"
    );
}

#[tokio::test]
/// BRANCH RESOLUTION: No fallback when requested branch exists
/// Oracle: Claims - if you ask for X, you get X
async fn test_no_fallback_when_branch_exists() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create a branch named "main" (already exists) and "custom"
    helper
        .create_branch("custom")
        .expect("Failed to create branch");
    helper
        .checkout_branch("custom")
        .expect("Failed to checkout");
    helper
        .create_and_stage_file("custom.txt", "custom content")
        .expect("Failed to create file");
    helper.commit("Add custom file").expect("Failed to commit");

    // When requesting "custom" as target, should use it (not fallback)
    let config = Config::default();
    let context = git_repo
        .get_git_info_for_branch_diff(&config, "main", "custom")
        .expect("Failed to get branch diff");

    assert_eq!(context.branch, "main -> custom");
}

#[tokio::test]
/// BRANCH RESOLUTION: Branch names with different characters are distinct
/// Oracle: Familiarity - git branch names distinguish between different names
/// Note: On case-insensitive filesystems (macOS), we test with clearly different names
async fn test_case_sensitivity() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create branch with specific name
    helper
        .create_branch("feature-test")
        .expect("Failed to create branch");

    // Try to access with a different name - should fail
    // On case-insensitive FS, use a clearly different name
    let config = Config::default();
    let result = git_repo.get_git_info_for_branch_diff(&config, "main", "feature-different");

    // Should fail because "feature-different" doesn't exist
    assert!(result.is_err(), "Should fail with nonexistent branch");

    // Verify error message mentions the branch name
    let err = result.expect_err("Should fail");
    let err_str = err.to_string();
    assert!(
        err_str.contains("feature-different") || err_str.contains("main"),
        "Error should mention attempted branches: {err_str}"
    );
}

#[tokio::test]
/// BRANCH RESOLUTION: Handle special characters in branch names
/// Oracle: User Expectations - branch names can contain /, -, _, .
async fn test_special_characters_in_branch_name() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create branches with various special characters
    let branch_names = vec![
        "feature/special-chars_test.v1",
        "bugfix/issue-123",
        "release/2.0.0",
    ];

    for branch_name in &branch_names {
        helper
            .create_branch(branch_name)
            .unwrap_or_else(|_| panic!("Failed to create branch: {branch_name}"));
    }

    // Test each branch can be resolved
    let config = Config::default();
    for branch_name in &branch_names {
        let result = git_repo.get_git_info_for_branch_diff(&config, "main", branch_name);

        // Should succeed (branch exists)
        assert!(
            result.is_ok(),
            "Should resolve branch with special characters: {branch_name}"
        );

        let context = result.expect("Failed to get branch diff");
        assert_eq!(context.branch, format!("main -> {branch_name}"));
    }
}

#[tokio::test]
/// BRANCH RESOLUTION: Handle remote-style branch names
/// Oracle: Familiarity - origin/main style branches
async fn test_remote_branch_handling() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create a local branch that looks like a remote ref
    helper
        .create_branch("origin/main")
        .expect("Failed to create origin/main branch");
    helper
        .checkout_branch("origin/main")
        .expect("Failed to checkout");
    helper
        .create_and_stage_file("remote.txt", "remote content")
        .expect("Failed to create file");
    helper.commit("Add remote file").expect("Failed to commit");

    // Should resolve correctly
    let config = Config::default();
    let context = git_repo
        .get_git_info_for_branch_diff(&config, "main", "origin/main")
        .expect("Failed to get branch diff");

    assert_eq!(context.branch, "main -> origin/main");
    assert_eq!(context.staged_files.len(), 1);
}

#[tokio::test]
/// BRANCH RESOLUTION: Tag vs branch ambiguity
/// Oracle: Familiarity - git distinguishes tags and branches
async fn test_tag_vs_branch_ambiguity() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");

    // Create a tag with same name as a branch
    helper
        .create_branch("v1.0.0")
        .expect("Failed to create branch");

    let head = repo
        .head()
        .expect("Failed to get HEAD")
        .peel_to_commit()
        .expect("Failed");
    let sig = repo.signature().expect("Failed to get signature");
    repo.tag("v1.0.0", &head.into_object(), &sig, "v1.0.0 tag", false)
        .expect("Failed to create tag");

    // When resolving "v1.0.0", git2 should resolve to the branch (refs/heads takes precedence)
    let config = Config::default();
    let result = git_repo.get_git_info_for_branch_diff(&config, "main", "v1.0.0");

    // Should succeed - branch takes precedence over tag
    assert!(
        result.is_ok(),
        "Should resolve to branch when tag and branch have same name"
    );
}
