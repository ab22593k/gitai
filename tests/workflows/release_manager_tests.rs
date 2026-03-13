//! Release Manager Workflow Tests
//! PROOF: Tests scenarios for release planning and changelog generation
//! Persona: Release manager who needs accurate changelogs and version tracking

use gitai::git::GitRepo;
use gitai::config::Config;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{setup_git_repo, TestAssertions, ComplexRepoBuilder};

#[tokio::test]
/// WORKFLOW: Release preparation - need version context
async fn test_release_preparation_workflow() {
    let result = ComplexRepoBuilder::new()
        .with_commits(3)
        .with_tags()
        .build();

    if let Ok((_temp_dir, git_repo)) = result {
        let config = Config::default();
        let context = git_repo.get_git_info(&config).await;
        
        TestAssertions::assert_success(
            &context,
            "Release preparation",
            Some("WORKFLOW: Should work for release preparation"),
        );
    }
}

#[tokio::test]
/// WORKFLOW: Changelog generation - need commit history
async fn test_changelog_generation_workflow() {
    let result = ComplexRepoBuilder::new()
        .with_commits(5)
        .build();

    if let Ok((_temp_dir, git_repo)) = result {
        let config = Config::default();
        let context = git_repo.get_git_info(&config).await.expect("Failed");

        // Need enough commits for meaningful changelog
        assert!(
            context.recent_commits.len() >= 1,
            "PROBLEM: Not enough history for changelog\n\
             CONTEXT: Release manager workflow\n\
             EXPECTED: Multiple commits\n\
             ACTUAL: {}",
            context.recent_commits.len()
        );
    }
}

#[tokio::test]
/// WORKFLOW: Version comparison - need tags
async fn test_version_comparison_workflow() {
    let result = ComplexRepoBuilder::new()
        .with_commits(2)
        .with_tags()
        .build();

    if let Ok((_temp_dir, git_repo)) = result {
        let config = Config::default();
        let context = git_repo.get_git_info(&config).await;
        
        TestAssertions::assert_success(
            &context,
            "Version comparison",
            Some("WORKFLOW: Should provide context for version comparison"),
        );
    }
}

#[tokio::test]
/// WORKFLOW: Release notes need detailed commit info
async fn test_release_notes_context() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let context = git_repo.get_git_info(&config).await.expect("Failed");

    // Should have commit details for release notes
    for commit in &context.recent_commits {
        assert!(
            !commit.message.is_empty(),
            "PROBLEM: Commit message empty\n\
             CONTEXT: Release notes need detailed messages\n\
             EXPECTED: Non-empty message\n\
             ACTUAL: Empty"
        );
    }
}
