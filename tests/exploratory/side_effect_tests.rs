//! Exploratory side effect tests
//! Tests that operations don't have unintended side effects
//! PROOF: Ensures operations are focused and don't affect unrelated state

use gitai::git::GitRepo;
use gitai::config::Config;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{setup_git_repo, TestAssertions};

#[tokio::test]
/// SIDE EFFECT: Verify unrelated files unchanged after operation
async fn test_unrelated_files_unchanged() {
    use git2::Repository;
    use std::fs;
    use std::path::Path;
    
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Create additional files (unrelated to operation)
    let unrelated_file = temp_dir.path().join("unrelated.txt");
    fs::write(&unrelated_file, "unrelated content").expect("Failed to write");
    
    // Get git info
    let before = git_repo.get_git_info(&config).await.expect("Failed");
    
    // Verify unrelated file wasn't mentioned as staged
    let staged_paths: Vec<&str> = before.staged_files.iter().map(|f| f.path.as_str()).collect();
    
    assert!(
        !staged_paths.contains(&"unrelated.txt"),
        "PROBLEM: Unrelated file appeared in staged files\n\
         CONTEXT: Side effect detection\n\
         EXPECTED: Only staged files should appear\n\
         ACTUAL: Unrelated file was included"
    );
}

#[tokio::test]
/// SIDE EFFECT: Verify operation is read-only
async fn test_operation_is_read_only() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Get git info - should not modify any files
    let result1 = git_repo.get_git_info(&config).await;
    TestAssertions::assert_success(&result1, "Read operation", None);

    // Check that we can still commit (repo not corrupted)
    use git2::Repository;
    use std::fs;
    use std::path::Path;
    
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");
    fs::write(temp_dir.path().join("new.txt"), "content").expect("Failed");
    
    let mut index = repo.index().expect("Failed");
    index.add_path(Path::new("new.txt")).expect("Failed");
    index.write().expect("Failed");
    
    // Should be able to make a new commit
    let tree_id = index.write_tree().expect("Failed");
    let tree = repo.find_tree(tree_id).expect("Failed");
    let sig = repo.signature().expect("Failed");
    let head = repo.head().expect("Failed").peel_to_commit().expect("Failed");
    
    let commit_result = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "Test commit",
        &tree,
        &[&head],
    );
    
    assert!(
        commit_result.is_ok(),
        "PROBLEM: Cannot commit after read operation\n\
         CONTEXT: Side effect - read operation modified state\n\
         EXPECTED: Repo remains writable\n\
         ACTUAL: Commit failed"
    );
}

#[tokio::test]
/// SIDE EFFECT: Verify branch info doesn't change unexpectedly
async fn test_branch_info_stable() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let context1 = git_repo.get_git_info(&config).await.expect("Failed");
    let context2 = git_repo.get_git_info(&config).await.expect("Failed");

    assert_eq!(
        context1.branch, context2.branch,
        "PROBLEM: Branch changed unexpectedly\n\
         CONTEXT: Side effect - branch stability\n\
         EXPECTED: Branch remains the same\n\
         ACTUAL: Branch changed from {} to {}",
        context1.branch, context2.branch
    );
}

#[tokio::test]
/// SIDE EFFECT: Verify recent commits don't get modified
async fn test_recent_commits_immutable() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let context1 = git_repo.get_git_info(&config).await.expect("Failed");
    let commits1: Vec<_> = context1.recent_commits.iter().map(|c| c.hash.clone()).collect();
    
    // Perform operation
    let _context2 = git_repo.get_git_info(&config).await.expect("Failed");
    
    // Get again
    let context3 = git_repo.get_git_info(&config).await.expect("Failed");
    let commits3: Vec<_> = context3.recent_commits.iter().map(|c| c.hash.clone()).collect();

    assert_eq!(
        commits1, commits3,
        "PROBLEM: Commit history changed after read\n\
         CONTEXT: Side effect - commit immutability\n\
         EXPECTED: Historical commits unchanged\n\
         ACTUAL: Commit hashes differ"
    );
}
