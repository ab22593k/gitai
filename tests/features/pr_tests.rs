use anyhow::Result;
use gitai::{
    commit::{
        prompt::{create_pr_system_prompt, create_pr_user_prompt},
        service::CommitService,
        types::{GeneratedPullRequest, format_pull_request},
    },
    config::Config,
    context::{ChangeType, CommitContext, StagedFile},
    git::GitRepo,
};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{MockDataBuilder, setup_git_repo_with_commits};

fn create_mock_pr_context() -> CommitContext {
    MockDataBuilder::pr_commit_context()
}

fn create_mock_generated_pr() -> GeneratedPullRequest {
    MockDataBuilder::generated_pull_request()
}

fn setup_test_repo_with_commits_arc() -> Result<(TempDir, Arc<GitRepo>)> {
    let (temp_dir, git_repo) = setup_git_repo_with_commits()?;
    Ok((temp_dir, Arc::new(git_repo)))
}

// Tests for PR prompt generation
#[test]
fn test_create_pr_system_prompt() {
    let config = Config::default();
    let prompt = create_pr_system_prompt(&config).expect("Failed to create PR system prompt");

    assert!(prompt.contains("pull request descriptions"));
    assert!(prompt.contains("atomic unit"));
    assert!(prompt.contains("title"));
    assert!(prompt.contains("summary"));
    assert!(prompt.contains("description"));
    assert!(prompt.contains("commits"));
    assert!(prompt.contains("breaking_changes"));
    assert!(prompt.contains("testing_notes"));
    assert!(prompt.contains("notes"));
}

#[test]
fn test_create_pr_system_prompt_with_custom_instructions() {
    let config = Config {
        instructions: "Always include security implications".to_string(),
        ..Default::default()
    };

    let prompt = create_pr_system_prompt(&config).expect("Failed to create PR system prompt");
    assert!(prompt.contains("Always include security implications"));
}

#[test]
fn test_create_pr_user_prompt_basic() {
    let context = create_mock_pr_context();
    let commit_messages = vec![
        "abc1234: Add JWT authentication middleware".to_string(),
        "def5678: Implement user registration endpoint".to_string(),
    ];

    let prompt = create_pr_user_prompt(&context, &commit_messages);

    assert!(prompt.contains("Range: main..feature-auth"));
    assert!(prompt.contains("Commits in this PR:"));
    assert!(prompt.contains("abc1234: Add JWT authentication middleware"));
    assert!(prompt.contains("def5678: Implement user registration endpoint"));
    assert!(prompt.contains("src/auth/middleware.rs"));
    assert!(prompt.contains("src/auth/models.rs"));
    assert!(prompt.contains("Language: Rust"));
    assert!(prompt.contains("Framework: Warp"));
}

#[test]
fn test_create_pr_user_prompt_empty_commits() {
    let context = create_mock_pr_context();
    let commit_messages = vec![];

    let prompt = create_pr_user_prompt(&context, &commit_messages);

    assert!(prompt.contains("No commits available"));
}

// Tests for PR type formatting
#[test]
fn test_format_pull_request() {
    let pr = create_mock_generated_pr();
    let formatted = format_pull_request(&pr);

    assert!(formatted.contains("# Add JWT authentication with user registration"));
    assert!(formatted.contains("## Summary"));
    assert!(formatted.contains("## Description"));
    assert!(formatted.contains("## Commits"));
    assert!(formatted.contains("- abc1234: Add JWT authentication middleware"));
    assert!(formatted.contains("- def5678: Implement user registration endpoint"));
    assert!(formatted.contains("## Breaking Changes"));
    assert!(formatted.contains("- All protected endpoints now require authentication headers"));
    assert!(formatted.contains("## Testing"));
    assert!(formatted.contains("Test user registration flow"));
    assert!(formatted.contains("## Notes"));
    assert!(formatted.contains("Requires JWT_SECRET environment variable"));
}

#[test]
fn test_format_pull_request_minimal() {
    let pr = GeneratedPullRequest {
        emoji: None,
        title: "Fix bug in user authentication".to_string(),
        summary: "Fixes a critical bug in the authentication flow".to_string(),
        description: "This PR fixes an issue where users couldn't log in properly.".to_string(),
        commits: vec!["abc1234: Fix authentication bug".to_string()],
        breaking_changes: vec![],
        testing_notes: None,
        notes: None,
    };

    let formatted = format_pull_request(&pr);

    assert!(formatted.contains("# Fix bug in user authentication"));
    assert!(formatted.contains("## Summary"));
    assert!(formatted.contains("## Description"));
    assert!(formatted.contains("## Commits"));
    assert!(formatted.contains("- abc1234: Fix authentication bug"));
    // Should not contain empty sections
    assert!(!formatted.contains("## Breaking Changes"));
    assert!(!formatted.contains("## Testing"));
    assert!(!formatted.contains("## Notes"));
}

// Tests for Git operations (using public API)
#[tokio::test]
async fn test_git_repo_get_commits_for_pr() -> Result<()> {
    let (temp_dir, git_repo) = setup_test_repo_with_commits_arc()?;
    let repo = git2::Repository::open(temp_dir.path())?;

    // Get commits between the initial commit and HEAD
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    let commits: Vec<_> = revwalk.collect::<std::result::Result<Vec<_>, _>>()?;

    if commits.len() >= 2 {
        let from_commit = repo.find_commit(commits[1])?; // Second commit (older)
        let to_commit = repo.find_commit(commits[0])?; // First commit (newer)

        let commit_messages = git_repo
            .get_commits_for_pr(&from_commit.id().to_string(), &to_commit.id().to_string())?;

        assert!(!commit_messages.is_empty());
        assert!(commit_messages[0].contains("Add main function"));
    }

    Ok(())
}

#[tokio::test]
async fn test_git_repo_get_commit_range_files() -> Result<()> {
    let (temp_dir, git_repo) = setup_test_repo_with_commits_arc()?;
    let repo = git2::Repository::open(temp_dir.path())?;

    // Get commits
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    let commits: Vec<_> = revwalk.collect::<std::result::Result<Vec<_>, _>>()?;

    if commits.len() >= 2 {
        let from_commit = repo.find_commit(commits[1])?; // Second commit (older)
        let to_commit = repo.find_commit(commits[0])?; // First commit (newer)

        let files = git_repo
            .get_commit_range_files(&from_commit.id().to_string(), &to_commit.id().to_string())?;

        assert!(!files.is_empty());
        assert!(files.iter().any(|f| f.path == "src/main.rs"));
        assert!(
            files
                .iter()
                .any(|f| matches!(f.change_type, ChangeType::Added))
        );
    }

    Ok(())
}

// Tests for service integration
#[tokio::test]
async fn test_service_pr_generation_setup() -> Result<()> {
    let (temp_dir, _git_repo) = setup_test_repo_with_commits_arc()?;
    let config = Config::default();
    let repo_path = PathBuf::from(temp_dir.path());
    let provider_name = "test";

    // Create a new GitRepo instance for the service
    let service_git_repo = GitRepo::new(temp_dir.path())?;

    let service = CommitService::new(
        config,
        &repo_path,
        provider_name,
        false, // emoji not needed for PR
        false, // verification not needed for PR
        service_git_repo,
    )?;

    // Test that service can be created successfully
    assert!(!service.is_remote_repository());

    Ok(())
}

// Integration tests for GitRepo PR methods
#[tokio::test]
async fn test_git_repo_pr_methods() -> Result<()> {
    let (temp_dir, git_repo) = setup_test_repo_with_commits_arc()?;
    let repo = git2::Repository::open(temp_dir.path())?;

    // Get commit IDs for testing
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    let commits: Vec<_> = revwalk.collect::<std::result::Result<Vec<_>, _>>()?;

    if commits.len() >= 2 {
        let from_commit = repo.find_commit(commits[1])?;
        let to_commit = repo.find_commit(commits[0])?;
        let from_id = from_commit.id().to_string();
        let to_id = to_commit.id().to_string();

        // Test get_commits_for_pr
        let commit_messages = git_repo.get_commits_for_pr(&from_id, &to_id)?;
        assert!(!commit_messages.is_empty());

        // Test get_commit_range_files
        let files = git_repo.get_commit_range_files(&from_id, &to_id)?;
        assert!(!files.is_empty());

        // Test get_git_info_for_commit_range
        let context =
            git_repo.get_git_info_for_commit_range(&Config::default(), &from_id, &to_id)?;
        assert!(context.branch.contains(".."));
        assert!(!context.staged_files.is_empty());
    }

    Ok(())
}

// Edge case tests
#[test]
fn test_pr_prompt_with_large_commit_list() {
    let context = create_mock_pr_context();
    let commit_messages: Vec<String> = (0..100)
        .map(|i| format!("abc{i:04}: Commit number {i}"))
        .collect();

    let prompt = create_pr_user_prompt(&context, &commit_messages);

    // Should handle large commit lists gracefully
    assert!(prompt.contains("Commits in this PR:"));
    assert!(prompt.contains("abc0000: Commit number 0"));
    assert!(prompt.contains("abc0099: Commit number 99"));
}

#[test]
fn test_format_pull_request_with_unicode() {
    let pr = GeneratedPullRequest {
        emoji: None,
        title: "Add 🚀 deployment automation".to_string(),
        summary: "Implements automated deployment with emojis 🎉".to_string(),
        description: "This PR adds deployment automation:\n\n• Feature 1\n• Feature 2 ✅"
            .to_string(),
        commits: vec!["abc1234: Add 🔧 configuration".to_string()],
        breaking_changes: vec!["⚠️ Configuration format changed".to_string()],
        testing_notes: Some("Test with 🧪 test suite".to_string()),
        notes: Some("Deployment requires 🔑 secrets".to_string()),
    };

    let formatted = format_pull_request(&pr);

    assert!(formatted.contains("🚀 deployment automation"));
    assert!(formatted.contains("🎉"));
    assert!(formatted.contains("✅"));
    assert!(formatted.contains("🔧"));
    assert!(formatted.contains("⚠️"));
    assert!(formatted.contains("🧪"));
    assert!(formatted.contains("🔑"));
}

#[test]
fn test_pr_context_with_excluded_files() {
    let mut context = create_mock_pr_context();
    context.staged_files.push(StagedFile {
        path: "node_modules/package/index.js".to_string(),
        change_type: ChangeType::Modified,
        diff: "[Content excluded]".to_string(),
        analysis: vec!["[Analysis excluded]".to_string()],
        content: None,
        content_excluded: true,
    });

    let commit_messages = vec!["abc1234: Update dependencies".to_string()];
    let prompt = create_pr_user_prompt(&context, &commit_messages);

    assert!(prompt.contains("node_modules/package/index.js"));
    assert!(prompt.contains("[Content excluded]"));
    assert!(prompt.contains("[Analysis excluded]"));
}

#[cfg(test)]
mod commitish_tests {
    // We need to expose the functions for testing
    // For now, let's create a simple test module that can test the logic

    /// Test helper to check if a reference looks like a commit hash
    fn is_likely_commit_hash(reference: &str) -> bool {
        reference.len() >= 7 && reference.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Test helper to check if a reference uses Git commitish syntax
    fn is_commitish_syntax(reference: &str) -> bool {
        reference.contains('~') || reference.contains('^') || reference.starts_with('@')
    }

    /// Test helper to check if a reference looks like a commit hash or commitish
    fn is_likely_commit_hash_or_commitish(reference: &str) -> bool {
        if reference.len() >= 7 && reference.chars().all(|c| c.is_ascii_hexdigit()) {
            return true;
        }
        is_commitish_syntax(reference)
    }

    #[test]
    fn test_commit_hash_detection() {
        assert!(is_likely_commit_hash("abcdef1234567"));
        assert!(is_likely_commit_hash("1234567"));
        assert!(!is_likely_commit_hash("abc123")); // Too short
        assert!(!is_likely_commit_hash("abcdefg1234567")); // Contains non-hex
        assert!(!is_likely_commit_hash("HEAD~2"));
        assert!(!is_likely_commit_hash("main"));
    }

    #[test]
    fn test_commitish_syntax_detection() {
        // Test tilde syntax
        assert!(is_commitish_syntax("HEAD~2"));
        assert!(is_commitish_syntax("main~1"));
        assert!(is_commitish_syntax("origin/main~3"));

        // Test caret syntax
        assert!(is_commitish_syntax("HEAD^"));
        assert!(is_commitish_syntax("HEAD^^"));
        assert!(is_commitish_syntax("main^2"));

        // Test @ syntax
        assert!(is_commitish_syntax("@"));
        assert!(is_commitish_syntax("@~3"));
        assert!(is_commitish_syntax("@^"));

        // Test combinations
        assert!(is_commitish_syntax("HEAD~2^"));

        // Test non-commitish
        assert!(!is_commitish_syntax("main"));
        assert!(!is_commitish_syntax("feature-branch"));
        assert!(!is_commitish_syntax("abcdef1234567"));
    }

    #[test]
    fn test_combined_detection() {
        // Commit hashes should be detected
        assert!(is_likely_commit_hash_or_commitish("abcdef1234567"));
        assert!(is_likely_commit_hash_or_commitish("1234567"));

        // Commitish syntax should be detected
        assert!(is_likely_commit_hash_or_commitish("HEAD~2"));
        assert!(is_likely_commit_hash_or_commitish("HEAD^"));
        assert!(is_likely_commit_hash_or_commitish("@~3"));
        assert!(is_likely_commit_hash_or_commitish("main~1"));

        // Regular branches should not be detected
        assert!(!is_likely_commit_hash_or_commitish("main"));
        assert!(!is_likely_commit_hash_or_commitish("feature-branch"));
        assert!(!is_likely_commit_hash_or_commitish("origin/main"));
    }
}
