use anyhow::Result;
use gait::{config::Config, features::commit::CommitService, git::GitRepo};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::setup_git_repo_with_commits;

fn setup_test_repo() -> Result<(TempDir, Arc<GitRepo>)> {
    let (temp_dir, git_repo) = setup_git_repo_with_commits()?;
    Ok((temp_dir, Arc::new(git_repo)))
}

#[tokio::test]
async fn test_perform_commit() -> Result<()> {
    let (temp_dir, _git_repo) = setup_test_repo()?;
    let config = Config::default();
    let repo_path = PathBuf::from(temp_dir.path());
    let provider_name = "test";
    let verify = true;

    // Create a new GitRepo for the service
    let service_repo = GitRepo::new(temp_dir.path())?;

    let service = CommitService::new(config, &repo_path, provider_name, verify, service_repo)?;

    let result = service.perform_commit("Test commit message", false, None)?;
    println!("Perform commit result: {result:?}");

    // Verify the commit was made
    let repo = git2::Repository::open(&repo_path)?;
    let head_commit = repo.head()?.peel_to_commit()?;
    assert_eq!(
        head_commit.message().expect("Failed to get commit message"),
        "Test commit message"
    );

    Ok(())
}

#[tokio::test]
async fn test_perform_amend_commit() -> Result<()> {
    let (temp_dir, _git_repo) = setup_test_repo()?;
    let config = Config::default();
    let repo_path = PathBuf::from(temp_dir.path());
    let provider_name = "test";
    let verify = true;

    // Create a new GitRepo for the service
    let service_repo = GitRepo::new(temp_dir.path())?;

    let service = CommitService::new(config, &repo_path, provider_name, verify, service_repo)?;

    // First, make an initial commit
    let result1 = service.perform_commit("Initial commit message", false, None)?;
    println!("Initial commit result: {result1:?}");

    // Now amend the commit
    let result2 = service.perform_commit("Amended commit message", true, Some("HEAD"))?;
    println!("Amend commit result: {result2:?}");

    // Verify the commit was amended
    let repo = git2::Repository::open(&repo_path)?;
    let head_commit = repo.head()?.peel_to_commit()?;
    assert_eq!(
        head_commit.message().expect("Failed to get commit message"),
        "Amended commit message"
    );

    // Verify the HEAD commit has the amended message
    let head_commit = repo.head()?.peel_to_commit()?;
    assert_eq!(
        head_commit.message().expect("Failed to get commit message"),
        "Amended commit message"
    );

    // Verify there are 3 commits total (2 from setup + 1 new + 1 amended replacing the new one)
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    let commits: Vec<_> = revwalk.collect();
    assert_eq!(commits.len(), 3);

    Ok(())
}
