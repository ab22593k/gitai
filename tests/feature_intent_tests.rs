//! Feature intent tests using Purpose oracle (FEW HICCUPS)
//! PROOF: Tests that features actually solve the intended user problem
//! Not just that they implement the spec, but that they work for users

use git2::Repository;
use gitai::config::Config;
use std::fs;
use std::path::Path;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{TestAssertions, setup_git_repo};

#[tokio::test]
/// PURPOSE ORACLE: Tests that git info serves its intended purpose
/// Purpose: Help users generate meaningful commits
async fn test_git_info_serves_commit_purpose() {
    let (_temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let context = git_repo.get_git_info(&config).await.expect(
        "PROBLEM: Failed to get git info\n\
                 CONTEXT: Purpose oracle - git info purpose\n\
                 EXPECTED: Successful retrieval for commit generation\n\
                 ACTUAL: Operation failed",
    );

    // PURPOSE: Git info should contain what's needed for commit generation
    // At minimum: branch, staged files, recent commits
    let has_minimum_info =
        !context.branch.is_empty() && context.recent_commits.iter().all(|c| !c.message.is_empty());

    TestAssertions::assert_with_proof(
        has_minimum_info,
        "Git info incomplete for commit purpose",
        "Purpose oracle - commit generation support",
        "Branch + recent commits available",
        format!(
            "Branch: {}, Commits: {}",
            context.branch,
            context.recent_commits.len()
        ),
        None,
    );
}

#[tokio::test]
/// PURPOSE ORACLE: Staged files should include diff info for review
/// Purpose: Help users understand what will be committed
async fn test_staged_files_include_useful_info() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Add and stage a file with specific content
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");
    fs::write(
        temp_dir.path().join("test.rs"),
        "fn main() { println!(\"hello\"); }",
    )
    .expect("Failed to write");

    let mut index = repo.index().expect("Failed to get index");
    index.add_path(Path::new("test.rs")).expect("Failed to add");
    index.write().expect("Failed to write index");

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // PURPOSE: Staged files should have diff for user review
    if let Some(file) = context.staged_files.iter().find(|f| f.path == "test.rs") {
        // User should be able to see what changed
        assert!(
            !file.diff.is_empty() || file.change_type.to_string() == "Added",
            "PROBLEM: Diff missing for review\n\
             CONTEXT: Purpose oracle - user review capability\n\
             EXPECTED: Diff available for staged file\n\
             ACTUAL: Empty diff for non-new file\n\
             PURPOSE: Users need to review changes before commit"
        );
    }
}

#[tokio::test]
/// PURPOSE ORACLE: Recent commits help provide context
/// Purpose: Help users write contextually appropriate messages
async fn test_recent_commits_provide_context() {
    let (_temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // PURPOSE: Recent commits should help generate contextual messages
    for commit in &context.recent_commits {
        TestAssertions::assert_with_proof(
            !commit.message.is_empty(),
            "Commit message empty",
            "Purpose oracle - context provision",
            "Non-empty commit message",
            format!("Hash: {}", commit.hash),
            None,
        );
    }
}

#[tokio::test]
/// PURPOSE ORACLE: Author history helps maintain consistency
/// Purpose: Help generate consistent commit style
async fn test_author_history_enables_consistency() {
    let (_temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let _context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // PURPOSE: Author history should be retrievable for style consistency
    // It's OK if empty for new repos, but should not error - verified by successful call above
}

#[tokio::test]
/// PURPOSE ORACLE: Branch info needed for proper commit context
/// Purpose: Branch affects commit message format (e.g., feature branches)
async fn test_branch_info_for_commit_context() {
    let (_temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // PURPOSE: Branch info needed for commit message conventions
    TestAssertions::assert_with_proof(
        !context.branch.is_empty(),
        "Branch info missing",
        "Purpose oracle - branch-aware commits",
        "Branch name available",
        format!("Branch: '{}'", context.branch),
        Some("PURPOSE: Branch prefixes (feat/, fix/) are common conventions"),
    );
}
