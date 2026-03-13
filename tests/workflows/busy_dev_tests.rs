//! Busy Developer Workflow Tests
//! PROOF: Tests common developer scenarios requiring minimal friction
//! Persona: Developer who wants quick, friction-free commits

use gitai::git::GitRepo;
use gitai::config::Config;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{setup_git_repo, TestAssertions};

#[tokio::test]
/// WORKFLOW: Quick commit flow - minimal friction
/// Busy developer wants: stage -> generate commit -> done
async fn test_quick_commit_flow() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Stage a file
    use git2::Repository;
    use std::fs;
    use std::path::Path;
    
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");
    fs::write(temp_dir.path().join("quick.rs"), "fn main() {}").expect("Failed");
    
    let mut index = repo.index().expect("Failed");
    index.add_path(Path::new("quick.rs")).expect("Failed");
    index.write().expect("Failed");

    // Get context - should be fast and complete
    let start = std::time::Instant::now();
    let context = git_repo.get_git_info(&config).await;
    let elapsed = start.elapsed();

    TestAssertions::assert_success(
        &context,
        "Quick commit flow",
        Some("WORKFLOW: Quick commit should work with minimal delay"),
    );

    // Should complete quickly (busy developer doesn't wait)
    assert!(
        elapsed.as_secs() < 5,
        "PROBLEM: Too slow for quick commit\n\
         CONTEXT: Busy developer workflow\n\
         EXPECTED: < 5 seconds\n\
         ACTUAL: {:?}",
        elapsed
    );
}

#[tokio::test]
/// WORKFLOW: Default config should work out of box
/// Busy developer doesn't want to configure things
async fn test_default_config_works() {
    let (temp_dir, git_repo) = setup_git_repo();
    
    // Default config should work without any setup
    let config = Config::default();
    
    let result = git_repo.get_git_info(&config).await;
    
    TestAssertions::assert_success(
        &result,
        "Default config workflow",
        Some("WORKFLOW: Default config should work immediately"),
    );
}

#[tokio::test]
/// WORKFLOW: Single file change - most common case
async fn test_single_file_change_workflow() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    use git2::Repository;
    use std::fs;
    use std::path::Path;
    
    let repo = Repository::open(temp_dir.path()).expect("Failed");
    fs::write(temp_dir.path().join("modified.txt"), "changed").expect("Failed");
    
    let mut index = repo.index().expect("Failed");
    index.add_path(Path::new("modified.txt")).expect("Failed");
    index.write().expect("Failed");

    let context = git_repo.get_git_info(&config).await.expect("Failed");

    // Should detect single file change
    assert_eq!(
        context.staged_files.len(),
        1,
        "PROBLEM: Single file change not detected\n\
         CONTEXT: Common workflow\n\
         EXPECTED: 1 staged file\n\
         ACTUAL: {}",
        context.staged_files.len()
    );
}

#[tokio::test]
/// WORKFLOW: View recent commits - common for context
async fn test_view_recent_commits_workflow() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let context = git_repo.get_git_info(&config).await.expect("Failed");

    // Recent commits should be available for context
    assert!(
        !context.recent_commits.is_empty(),
        "PROBLEM: Cannot view recent commits\n\
         CONTEXT: Developer workflow\n\
         EXPECTED: At least initial commit visible\n\
         ACTUAL: Empty"
    );
}
