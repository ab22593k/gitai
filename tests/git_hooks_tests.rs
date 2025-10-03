// only run this test on Linux
#![cfg(target_os = "linux")]
use anyhow::Result;
use git2::Repository;

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{GitHooksTestHelper, GitTestHelper, setup_git_repo};

#[test]
fn test_verify_and_commit_success() -> Result<()> {
    let (temp_dir, git_repo) = setup_git_repo();
    let repo_path = temp_dir.path();

    // Create successful pre-commit and post-commit hooks using our helper
    GitHooksTestHelper::create_hook(
        repo_path,
        "pre-commit",
        "echo \"Pre-commit checks passed\"",
        false,
    )?;
    GitHooksTestHelper::create_hook(
        repo_path,
        "post-commit",
        "echo \"Post-commit tasks completed\"",
        false,
    )?;

    // Create and stage a new file using our helper
    let helper = GitTestHelper::new(&temp_dir)?;
    helper.create_and_stage_file("test_file.txt", "Test content")?;

    let precommit = git_repo.execute_hook("pre-commit");
    assert!(precommit.is_ok(), "Pre-commit hook should succeed");

    // Perform commit_and_verify
    let result = git_repo.commit_and_verify("Test commit message");

    assert!(result.is_ok(), "verify_and_commit should succeed");
    let commit_result = result.expect("Commit failed");
    assert_eq!(commit_result.files_changed, 1);
    assert!(!commit_result.commit_hash.is_empty());

    Ok(())
}

#[test]
fn test_verify_and_commit_pre_commit_failure() -> Result<()> {
    let (temp_dir, git_repo) = setup_git_repo();
    let repo_path = temp_dir.path();

    // Create a failing pre-commit hook using our helper
    GitHooksTestHelper::create_hook(
        repo_path,
        "pre-commit",
        "echo \"Pre-commit checks failed\"",
        true,
    )?;

    // Create and stage a new file using our helper
    let helper = GitTestHelper::new(&temp_dir)?;
    helper.create_and_stage_file("test_file.txt", "Test content")?;

    let precommit = git_repo.execute_hook("pre-commit");
    assert!(
        precommit.is_err(),
        "Commit should fail due to pre-commit hook"
    );

    // Verify that no commit was made
    let repo = Repository::open(repo_path).expect("Failed to open repository");
    let head_commit = repo.head()?.peel_to_commit()?;
    assert_eq!(
        head_commit.message().expect("Failed to get commit message"),
        "Initial commit"
    );

    Ok(())
}

#[test]
fn test_verify_and_commit_post_commit_failure() -> Result<()> {
    let (temp_dir, git_repo) = setup_git_repo();
    let repo_path = temp_dir.path();

    // Create successful pre-commit and failing post-commit hooks using our helper
    GitHooksTestHelper::create_hook(
        repo_path,
        "pre-commit",
        "echo \"Pre-commit checks passed\"",
        false,
    )?;
    GitHooksTestHelper::create_hook(
        repo_path,
        "post-commit",
        "echo \"Post-commit tasks failed\"",
        true,
    )?;

    // Create and stage a new file using our helper
    let helper = GitTestHelper::new(&temp_dir)?;
    helper.create_and_stage_file("test_file.txt", "Test content")?;

    let precommit = git_repo.execute_hook("pre-commit");
    assert!(precommit.is_ok(), "Pre-commit hook should succeed");

    // Perform commit_and_verify
    let result = git_repo.commit_and_verify("Test commit message");

    // The commit should succeed even if the post-commit hook fails
    assert!(
        result.is_ok(),
        "verify_and_commit should succeed despite post-commit hook failure"
    );
    let commit_result = result.expect("Commit failed");
    assert_eq!(commit_result.files_changed, 1);
    assert!(!commit_result.commit_hash.is_empty());

    // Verify that the commit was made
    let repo = Repository::open(repo_path).expect("Failed to open repository");
    let head_commit = repo.head()?.peel_to_commit()?;
    assert_eq!(
        head_commit.message().expect("Failed to get commit message"),
        "Test commit message"
    );

    Ok(())
}

#[test]
fn test_verify_and_commit_no_hooks() -> Result<()> {
    let (temp_dir, git_repo) = setup_git_repo();
    let repo_path = temp_dir.path();

    // Create and stage a new file using our helper
    let helper = GitTestHelper::new(&temp_dir)?;
    helper.create_and_stage_file("test_file.txt", "Test content")?;

    let precommit = git_repo.execute_hook("pre-commit");
    assert!(precommit.is_ok(), "Pre-commit hook should succeed");

    // Perform commit_and_verify
    let result = git_repo.commit_and_verify("Test commit message");

    assert!(
        result.is_ok(),
        "verify_and_commit should succeed without hooks"
    );
    let commit_result = result.expect("Commit failed");
    assert_eq!(commit_result.files_changed, 1);
    assert!(!commit_result.commit_hash.is_empty());

    // Verify that the commit was made
    let repo = Repository::open(repo_path).expect("Failed to open repository");
    let head_commit = repo.head()?.peel_to_commit()?;
    assert_eq!(
        head_commit.message().expect("Failed to get commit message"),
        "Test commit message"
    );

    Ok(())
}
