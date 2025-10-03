use anyhow::Result;
use gitpilot::{commit::CommitService, config::Config, git::GitRepo};
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
    let use_emoji = true;
    let verify = true;

    // Create a new GitRepo for the service
    let service_repo = GitRepo::new(temp_dir.path())?;

    let service = CommitService::new(
        config,
        &repo_path,
        provider_name,
        use_emoji,
        verify,
        service_repo,
    )?;

    let result = service.perform_commit("Test commit message")?;
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
