// only run this test on Linux
#![cfg(target_os = "linux")]

use anyhow::Result;
use gitai::git::GitRepo;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{GitTestHelper, setup_git_repo, setup_git_repo_with_tags};

#[test]
fn test_get_latest_tag_returns_some_when_tags_exist() -> Result<()> {
    let (temp_dir, _) = setup_git_repo_with_tags()?;
    let git_repo = GitRepo::new(temp_dir.path())?;

    let latest_tag = git_repo.get_latest_tag()?;
    assert!(latest_tag.is_some(), "Expected a tag, but got None");
    assert_eq!(latest_tag.expect("Tag should be Some"), "v1.1.0");

    Ok(())
}

#[test]
fn test_get_latest_tag_returns_none_when_no_tags() -> Result<()> {
    let (_temp_dir, git_repo) = setup_git_repo();

    let latest_tag = git_repo.get_latest_tag()?;
    assert!(
        latest_tag.is_none(),
        "Expected None for repository without tags"
    );

    Ok(())
}

#[test]
fn test_get_first_commit_returns_commit_hash() -> Result<()> {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir)?;

    helper.create_and_stage_file("file2.txt", "content")?;
    helper.commit("Second commit")?;

    let first_commit = git_repo.get_first_commit()?;
    assert!(!first_commit.is_empty(), "Expected a commit hash");

    Ok(())
}

#[test]
fn test_get_first_commit_in_repo_with_initial_commit() -> Result<()> {
    let (_temp_dir, git_repo) = setup_git_repo();

    let first_commit = git_repo.get_first_commit()?;
    assert!(!first_commit.is_empty(), "Expected a commit hash");

    Ok(())
}
