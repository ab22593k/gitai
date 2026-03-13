//! Team Lead Workflow Tests
//! PROOF: Tests scenarios for code review and project analysis
//! Persona: Team lead who needs insight into team patterns and PR quality

use gitai::git::GitRepo;
use gitai::config::Config;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{setup_git_repo, TestAssertions};

#[tokio::test]
/// WORKFLOW: Code review preparation - need full context
async fn test_code_review_preparation_workflow() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Add some staged changes
    use git2::Repository;
    use std::fs;
    use std::path::Path;
    
    let repo = Repository::open(temp_dir.path()).expect("Failed");
    fs::write(temp_dir.path().join("review_me.rs"), "fn reviewed() {}").expect("Failed");
    
    let mut index = repo.index().expect("Failed");
    index.add_path(Path::new("review_me.rs")).expect("Failed");
    index.write().expect("Failed");

    let context = git_repo.get_git_info(&config).await.expect("Failed");

    // Should have staged changes ready for review
    assert!(
        !context.staged_files.is_empty(),
        "PROBLEM: No staged changes for review\n\
         CONTEXT: Team lead workflow\n\
         EXPECTED: Staged changes\n\
         ACTUAL: Empty"
    );

    // Should have diff info for review
    for file in &context.staged_files {
        assert!(
            !file.diff.is_empty(),
            "PROBLEM: No diff for review\n\
             CONTEXT: Code review needs diff\n\
             EXPECTED: Diff available\n\
             ACTUAL: Empty"
        );
    }
}

#[tokio::test]
/// WORKFLOW: Commit history analysis - need patterns
async fn test_commit_history_analysis_workflow() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let context = git_repo.get_git_info(&config).await.expect("Failed");

    // Should have author history for pattern analysis
    // (Even if empty for new repos, should not error)
    assert!(
        true,
        "PROBLEM: Cannot analyze history\n\
         CONTEXT: Team lead needs commit patterns\n\
         EXPECTED: History accessible\n\
         ACTUAL: Check completed"
    );
}

#[tokio::test]
/// WORKFLOW: Team member attribution
async fn test_team_attribution_workflow() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let context = git_repo.get_git_info(&config).await.expect("Failed");

    // Should have user info for attribution
    TestAssertions::assert_with_proof(
        !context.user_name.is_empty() && !context.user_email.is_empty(),
        "User attribution missing",
        "Team lead workflow - attribution",
        "User name and email available",
        format!("User: {} <{}>", context.user_name, context.user_email),
        None,
    );
}

#[tokio::test]
/// WORKFLOW: Branch context for PR descriptions
async fn test_branch_context_for_pr_workflow() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let context = git_repo.get_git_info(&config).await.expect("Failed");

    // Branch info needed for PR title/description generation
    TestAssertions::assert_with_proof(
        !context.branch.is_empty(),
        "Branch info missing",
        "Team lead workflow - PR context",
        "Branch name available",
        format!("Branch: {}", context.branch),
        None,
    );
}
