use git2::Repository;
use gitai::{
    config::{Config, ProviderConfig},
    core::context::{ChangeType, CommitContext, RecentCommit, StagedFile},
    features::{
        changelog::{
            change_analyzer::{AnalyzedChange, FileChange},
            models::{ChangeMetrics, ChangelogType},
        },
        commit::types::GeneratedPullRequest,
    },
    git::GitRepo,
};

use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Creates a temporary Git repository with an initial commit for testing
#[allow(dead_code)]
pub fn setup_git_repo() -> (TempDir, GitRepo) {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let repo = Repository::init(temp_dir.path()).expect("Failed to initialize repository");

    // Configure git user
    let mut config = repo.config().expect("Failed to get repository config");
    config
        .set_str("user.name", "Test User")
        .expect("Failed to set user name");
    config
        .set_str("user.email", "test@example.com")
        .expect("Failed to set user email");

    // Create and commit an initial file
    let initial_file_path = temp_dir.path().join("initial.txt");
    fs::write(&initial_file_path, "Initial content").expect("Failed to write initial file");

    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("initial.txt"))
        .expect("Failed to add file to index");
    index.write().expect("Failed to write index");

    let tree_id = index.write_tree().expect("Failed to write tree");
    let tree = repo.find_tree(tree_id).expect("Failed to find tree");
    let signature = repo.signature().expect("Failed to create signature");
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )
    .expect("Failed to commit");

    // Ensure the default branch is named 'main' for consistency across environments
    {
        let head_commit = repo
            .head()
            .expect("Failed to get HEAD")
            .peel_to_commit()
            .expect("Failed to peel HEAD to commit");
        let current_branch = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(std::string::ToString::to_string))
            .unwrap_or_default();
        if current_branch != "main" {
            // Create or update the 'main' branch pointing to the current HEAD commit
            repo.branch("main", &head_commit, true)
                .expect("Failed to create 'main' branch");
            repo.set_head("refs/heads/main")
                .expect("Failed to set HEAD to 'main' branch");
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .expect("Failed to checkout 'main' branch");
        }
    }

    let git_repo = GitRepo::new(temp_dir.path()).expect("Failed to create GitRepo");
    (temp_dir, git_repo)
}

/// Creates a Git repository with tags for changelog/release notes testing
#[allow(dead_code)]
pub fn setup_git_repo_with_tags() -> Result<(TempDir, Repository)> {
    let temp_dir = TempDir::new()?;
    let repo = Repository::init(temp_dir.path())?;

    let signature = git2::Signature::now("Test User", "test@example.com")?;

    // Create initial commit
    {
        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )?;
    }

    // Create a tag for the initial commit (v1.0.0)
    {
        let head = repo.head()?.peel_to_commit()?;
        repo.tag(
            "v1.0.0",
            &head.into_object(),
            &signature,
            "Version 1.0.0",
            false,
        )?;
    }

    // Create a new file and commit
    fs::write(temp_dir.path().join("file1.txt"), "Hello, world!")?;
    {
        let mut index = repo.index()?;
        index.add_path(Path::new("file1.txt"))?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let parent = repo.head()?.peel_to_commit()?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Add file1.txt",
            &tree,
            &[&parent],
        )?;
    }

    // Create another tag (v1.1.0)
    {
        let head = repo.head()?.peel_to_commit()?;
        repo.tag(
            "v1.1.0",
            &head.into_object(),
            &signature,
            "Version 1.1.0",
            false,
        )?;
    }

    Ok((temp_dir, repo))
}

/// Creates a Git repository with multiple commits for PR testing
#[allow(dead_code)]
pub fn setup_git_repo_with_commits() -> Result<(TempDir, GitRepo)> {
    let temp_dir = TempDir::new()?;
    let repo = Repository::init(temp_dir.path())?;

    // Configure git user
    let mut config = repo.config()?;
    config.set_str("user.name", "Test User")?;
    config.set_str("user.email", "test@example.com")?;

    // Create initial commit
    let signature = git2::Signature::now("Test User", "test@example.com")?;

    // Create initial file
    fs::write(temp_dir.path().join("README.md"), "# Initial Project")?;
    let mut index = repo.index()?;
    index.add_path(Path::new("README.md"))?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let initial_commit = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )?;

    // Create src directory and second commit
    fs::create_dir_all(temp_dir.path().join("src"))?;
    fs::write(
        temp_dir.path().join("src/main.rs"),
        "fn main() { println!(\"Hello\"); }",
    )?;
    index.add_path(Path::new("src/main.rs"))?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let parent_commit = repo.find_commit(initial_commit)?;
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Add main function",
        &tree,
        &[&parent_commit],
    )?;

    let git_repo = GitRepo::new(temp_dir.path())?;
    Ok((temp_dir, git_repo))
}

/// Creates a minimal temporary directory with just a `GitRepo` (no git initialization)
#[allow(dead_code)]
pub fn setup_temp_dir() -> (TempDir, GitRepo) {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let git_repo = GitRepo::new(temp_dir.path()).expect("Failed to create GitRepo");
    (temp_dir, git_repo)
}

/// Git repository operations helper
#[allow(dead_code)]
pub struct GitTestHelper<'a> {
    pub temp_dir: &'a TempDir,
    pub repo: Repository,
}

#[allow(dead_code)]
impl<'a> GitTestHelper<'a> {
    pub fn new(temp_dir: &'a TempDir) -> Result<Self> {
        let repo = Repository::open(temp_dir.path())?;
        Ok(Self { temp_dir, repo })
    }

    /// Create and stage a file
    pub fn create_and_stage_file(&self, path: &str, content: &str) -> Result<()> {
        let file_path = self.temp_dir.path().join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, content)?;

        let mut index = self.repo.index()?;
        index.add_path(Path::new(path))?;
        index.write()?;
        Ok(())
    }

    /// Create a commit with the staged files
    pub fn commit(&self, message: &str) -> Result<git2::Oid> {
        let mut index = self.repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;
        let signature = self.repo.signature()?;

        let parent_commit = if let Ok(head) = self.repo.head() {
            Some(head.peel_to_commit()?)
        } else {
            None
        };

        let parents: Vec<&git2::Commit> = parent_commit.as_ref().into_iter().collect();

        Ok(self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )?)
    }

    /// Create a new branch
    pub fn create_branch(&self, name: &str) -> Result<()> {
        let head_commit = self.repo.head()?.peel_to_commit()?;
        self.repo.branch(name, &head_commit, false)?;
        Ok(())
    }

    /// Switch to a branch
    pub fn checkout_branch(&self, name: &str) -> Result<()> {
        let branch = self.repo.find_branch(name, git2::BranchType::Local)?;
        let branch_name = branch
            .get()
            .name()
            .expect("Branch should have a valid name");
        self.repo.set_head(branch_name)?;
        self.repo
            .checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        Ok(())
    }

    /// Create a tag
    pub fn create_tag(&self, name: &str, message: &str) -> Result<()> {
        let head = self.repo.head()?.peel_to_commit()?;
        let signature = self.repo.signature()?;
        self.repo
            .tag(name, &head.into_object(), &signature, message, false)?;
        Ok(())
    }
}

// Mock data creators
#[allow(dead_code)]
pub struct MockDataBuilder;

#[allow(dead_code)]
impl MockDataBuilder {
    /// Create a mock `CommitContext` for testing
    pub fn commit_context() -> CommitContext {
        CommitContext {
            branch: "main".to_string(),
            recent_commits: vec![RecentCommit {
                hash: "abcdef1".to_string(),
                message: "Initial commit".to_string(),
                timestamp: "1234567890".to_string(),
            }],
            staged_files: vec![Self::staged_file()],
            user_name: "Test User".to_string(),
            user_email: "test@example.com".to_string(),
            author_history: vec!["feat: add user authentication".to_string()],
        }
    }

    /// Create a mock `CommitContext` for PR testing
    pub fn pr_commit_context() -> CommitContext {
        CommitContext {
            branch: "main..feature-auth".to_string(),
            recent_commits: vec![
                RecentCommit {
                    hash: "abc1234".to_string(),
                    message: "Add JWT authentication middleware".to_string(),
                    timestamp: "1234567890".to_string(),
                },
                RecentCommit {
                    hash: "def5678".to_string(),
                    message: "Implement user registration endpoint".to_string(),
                    timestamp: "1234567891".to_string(),
                },
            ],
            staged_files: vec![
                StagedFile {
                    path: "src/auth/middleware.rs".to_string(),
                    change_type: ChangeType::Added,
                    diff: "+ use jwt::encode;\n+ pub fn auth_middleware() -> impl Filter<Extract = (), Error = Rejection> + Clone {".to_string(),
                    content: Some("use jwt::encode;\n\npub fn auth_middleware() -> impl Filter {}".to_string()),
                    content_excluded: false,
                },
                StagedFile {
                    path: "src/auth/models.rs".to_string(),
                    change_type: ChangeType::Added,
                    diff: "+ #[derive(Serialize, Deserialize)]\n+ pub struct User {".to_string(),
                    content: Some("#[derive(Serialize, Deserialize)]\npub struct User {\n    pub id: u32,\n    pub email: String,\n}".to_string()),
                    content_excluded: false,
                },
            ],


            user_name: "Test User".to_string(),
            user_email: "test@example.com".to_string(),
            author_history: vec![
                "feat: add JWT authentication".to_string(),
                "fix: resolve token validation bug".to_string(),
            ],
        }
    }

    /// Create a mock `StagedFile`
    pub fn staged_file() -> StagedFile {
        StagedFile {
            path: "file1.rs".to_string(),
            change_type: ChangeType::Modified,
            diff: "- old line\n+ new line".to_string(),
            content: None,
            content_excluded: false,
        }
    }

    /// Create a mock `StagedFile` with specific properties
    pub fn staged_file_with(path: &str, change_type: ChangeType, diff: &str) -> StagedFile {
        StagedFile {
            path: path.to_string(),
            change_type,
            diff: diff.to_string(),
            content: None,
            content_excluded: false,
        }
    }

    /// Create a mock `StagedFile` for analysis testing (empty analysis initially)
    pub fn staged_file_for_analysis(path: &str, change_type: ChangeType, diff: &str) -> StagedFile {
        Self::staged_file_with(path, change_type, diff)
    }

    /// Create a mock Config
    pub fn config() -> Config {
        Config::default()
    }

    /// Create a mock Config with custom instructions
    pub fn config_with_instructions(instructions: &str) -> Config {
        Config {
            instructions: instructions.to_string(),
            ..Default::default()
        }
    }

    /// Create a mock test Config with API key
    pub fn test_config_with_api_key(provider: &str, api_key: &str) -> Config {
        let provider_config = ProviderConfig {
            api_key: api_key.to_string(),
            model_name: "test-model".to_string(),
            ..Default::default()
        };

        Config {
            providers: [(provider.to_string(), provider_config)]
                .into_iter()
                .collect(),
            ..Default::default()
        }
    }

    /// Create mock `AnalyzedChange` for changelog testing
    pub fn analyzed_change() -> AnalyzedChange {
        AnalyzedChange {
            commit_hash: "abcdef123456".to_string(),
            commit_message: "Add new feature".to_string(),
            file_changes: vec![FileChange {
                old_path: "src/old.rs".to_string(),
                new_path: "src/new.rs".to_string(),
                change_type: ChangeType::Modified,
                analysis: vec!["Modified function: process_data".to_string()],
            }],
            metrics: Self::change_metrics(),
            impact_score: 0.75,
            change_type: ChangelogType::Added,
            is_breaking_change: false,
            associated_issues: vec!["#123".to_string()],
            pull_request: Some("PR #456".to_string()),
        }
    }

    /// Create mock `ChangeMetrics`
    pub fn change_metrics() -> ChangeMetrics {
        ChangeMetrics {
            total_commits: 1,
            files_changed: 1,
            insertions: 15,
            deletions: 5,
            total_lines_changed: 20,
        }
    }

    /// Create mock total `ChangeMetrics`
    pub fn total_change_metrics() -> ChangeMetrics {
        ChangeMetrics {
            total_commits: 5,
            files_changed: 10,
            insertions: 100,
            deletions: 50,
            total_lines_changed: 150,
        }
    }

    /// Create a mock `GeneratedPullRequest`
    pub fn generated_pull_request() -> GeneratedPullRequest {
        GeneratedPullRequest {
            title: "Add JWT authentication with user registration".to_string(),
            summary: "Implements comprehensive JWT-based authentication system with user registration, login, and middleware for protected routes.".to_string(),
            description: "This PR introduces a complete authentication system:\n\n**Features Added:**\n- JWT token generation and validation\n- User registration endpoint\n- Authentication middleware for protected routes\n- Password hashing with bcrypt\n\n**Technical Details:**\n- Uses industry-standard JWT libraries\n- Implements secure password storage\n- Includes comprehensive error handling".to_string(),
            commits: vec![
                "abc1234: Add JWT authentication middleware".to_string(),
                "def5678: Implement user registration endpoint".to_string(),
            ],
            breaking_changes: vec![
                "All protected endpoints now require authentication headers".to_string(),
            ],
            testing_notes: Some("Test user registration flow and verify JWT tokens are properly validated on protected routes.".to_string()),
            notes: Some("Requires JWT_SECRET environment variable to be set before deployment.".to_string()),
        }
    }

    /// Create a mock binary file for testing
    pub fn mock_binary_content() -> Vec<u8> {
        vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ]
    }
}

/// Test assertion helpers with PROOF heuristic:
/// P - Problem: Clear description of what went wrong
/// R - Reproduction: Steps to reproduce the issue
/// O - Outcome: Actual vs. expected behavior
/// O - Other evidence: Logs, state snapshots
/// F - Frequency: How often this occurs
#[allow(dead_code)]
pub struct TestAssertions;

#[allow(dead_code)]
impl TestAssertions {
    /// Assert that a commit context has expected properties
    /// PROOF: Tests fundamental data integrity
    pub fn assert_commit_context_basics(context: &CommitContext) {
        assert!(
            !context.branch.is_empty(),
            "PROBLEM: Branch name is empty\n\
             CONTEXT: CommitContext validation\n\
             EXPECTED: Non-empty branch name\n\
             ACTUAL: Empty string\n\
             EVIDENCE: context.branch = \"{}\"",
            context.branch
        );
        assert!(
            !context.user_name.is_empty(),
            "PROBLEM: User name is empty\n\
             CONTEXT: CommitContext validation\n\
             EXPECTED: Non-empty user name\n\
             ACTUAL: Empty string\n\
             EVIDENCE: context.user_name = \"{}\"",
            context.user_name
        );
        assert!(
            !context.user_email.is_empty(),
            "PROBLEM: User email is empty\n\
             CONTEXT: CommitContext validation\n\
             EXPECTED: Non-empty user email\n\
             ACTUAL: Empty string\n\
             EVIDENCE: context.user_email = \"{}\"",
            context.user_email
        );
    }

    /// Assert that staged files contain expected changes
    /// PROOF: Tests that git detected changes correctly
    pub fn assert_staged_files_not_empty(context: &CommitContext) {
        assert!(
            !context.staged_files.is_empty(),
            "PROBLEM: No staged files found\n\
             CONTEXT: Expected staged changes\n\
             EXPECTED: At least one staged file\n\
             ACTUAL: Empty staged files list\n\
             FREQUENCY: Always if files are staged"
        );
    }

    /// Assert that a string contains emoji
    /// PROOF: Tests user expectation - emoji are standard in modern commits
    pub fn assert_contains_emoji(text: &str) {
        let emoji_chars = ["✨", "🐛", "📝", "💄", "♻️", "✅", "🔨"];
        assert!(
            emoji_chars.iter().any(|&emoji| text.contains(emoji)),
            "PROBLEM: Text missing expected emoji\n\
             CONTEXT: User expectation for emoji in commit messages\n\
             EXPECTED: At least one emoji from: {:?}\n\
             ACTUAL: No emoji found in text\n\
             EVIDENCE: text = \"{}\"",
            emoji_chars,
            &text[..text.len().min(100)]
        );
    }

    /// Assert that a prompt contains essential commit information
    /// PROOF: Tests completeness - missing info causes poor commits
    pub fn assert_commit_prompt_essentials(prompt: &str) {
        assert!(
            prompt.contains("Branch:"),
            "PROBLEM: Prompt missing branch information\n\
             CONTEXT: Commit prompt generation\n\
             EXPECTED: Prompt contains 'Branch:'\n\
             ACTUAL: Branch info not found\n\
             FREQUENCY: Always on branch info failure"
        );
        assert!(
            prompt.contains("commit"),
            "PROBLEM: Prompt missing commit information\n\
             CONTEXT: Commit prompt generation\n\
             EXPECTED: Prompt mentions commits\n\
             ACTUAL: No commit info found\n\
             FREQUENCY: Always when commits missing"
        );
    }

    /// Assert that token count is within limit
    /// PROOF: Tests requirement - tokens over limit cause API failures
    pub fn assert_token_limit(actual: usize, limit: usize) {
        assert!(
            actual <= limit,
            "PROBLEM: Token count exceeds limit\n\
             CONTEXT: Prompt size validation\n\
             EXPECTED: {actual} <= {limit}\n\
             ACTUAL: {actual} > {limit}\n\
             FREQUENCY: Depends on staged file count"
        );
    }

    /// Assert two values are equal with detailed PROOF message
    /// PROOF: Generic equality assertion with full context
    #[allow(clippy::needless_pass_by_value)]
    pub fn assert_eq<T>(actual: T, expected: T, context: &str, evidence: Option<&str>)
    where
        T: std::fmt::Debug + std::fmt::Display + PartialEq,
    {
        assert!(
            actual == expected,
            "PROBLEM: Values do not match\n\
             CONTEXT: {}\n\
             EXPECTED: {}\n\
             ACTUAL: {}{}",
            context,
            expected,
            actual,
            evidence
                .map(|e| format!("\nEVIDENCE: {e}"))
                .unwrap_or_default()
        );
    }

    /// Assert operation succeeded with detailed error context
    /// PROOF: Generic Result assertion for operation outcomes
    pub fn assert_success<T, E: std::fmt::Debug>(
        result: &Result<T, E>,
        operation: &str,
        evidence: Option<&str>,
    ) {
        assert!(
            result.is_ok(),
            "PROBLEM: Operation failed\n\
             CONTEXT: {}\n\
             EXPECTED: Success\n\
             ACTUAL: {:?}{}",
            operation,
            result.as_ref().err(),
            evidence
                .map(|e| format!("\nEVIDENCE: {e}"))
                .unwrap_or_default()
        );
    }

    /// Assert operation failed with expected error type
    /// PROOF: Tests error handling paths
    pub fn assert_failure<T, E: std::fmt::Debug>(
        result: &Result<T, E>,
        operation: &str,
        expected_error_contains: Option<&str>,
    ) {
        assert!(
            result.is_err(),
            "PROBLEM: Operation succeeded unexpectedly\n\
             CONTEXT: {operation} should have failed\n\
             EXPECTED: Failure with error containing '{expected_error_contains:?}'\n\
             ACTUAL: Success\n\
             FREQUENCY: Unexpected success indicates logic error"
        );

        if let Some(expected) = expected_error_contains {
            let err = format!("{:?}", result.as_ref().err());
            assert!(
                err.contains(expected),
                "PROBLEM: Error message mismatch\n\
                 CONTEXT: {operation} failure\n\
                 EXPECTED: Error containing '{expected}'\n\
                 ACTUAL: {err}"
            );
        }
    }

    /// Assert condition with comprehensive PROOF message
    /// PROOF: Generic condition assertion
    pub fn assert_with_proof(
        condition: bool,
        problem: &str,
        context: &str,
        expected: &str,
        actual: impl AsRef<str>,
        evidence: Option<&str>,
    ) {
        let actual_str = actual.as_ref();
        assert!(
            condition,
            "PROBLEM: {}\n\
             CONTEXT: {}\n\
             EXPECTED: {}\n\
             ACTUAL: {}{}",
            problem,
            context,
            expected,
            actual_str,
            evidence
                .map(|e| format!("\nEVIDENCE: {e}"))
                .unwrap_or_default()
        );
    }

    /// Oracle: Familiarity - Compare branch format with git CLI output
    /// PROOF: Verifies compatibility with familiar git behavior
    pub fn assert_branch_format_matches_git(branch: &str) {
        assert!(
            !branch.is_empty(),
            "PROBLEM: Branch name is empty\n\
             CONTEXT: Oracle - Familiarity (git branch format)\n\
             EXPECTED: Non-empty branch name like git CLI\n\
             ACTUAL: Empty string\n\
             FREQUENCY: Always on branch resolution failure"
        );

        // Git branch names can contain alphanumeric, -, /, ., _
        // But shouldn't start with - or contain certain special chars
        assert!(
            !branch.starts_with('-') && !branch.starts_with('/'),
            "PROBLEM: Branch name starts with invalid character\n\
             CONTEXT: Oracle - Familiarity (git naming rules)\n\
             EXPECTED: Valid git branch name\n\
             ACTUAL: '{branch}'"
        );
    }

    /// Oracle: Standards - Verify error includes fallback context
    /// PROOF: Error messages should guide user to resolution
    pub fn assert_error_includes_suggestions(err: &str, attempted: &[&str]) {
        assert!(
            attempted.iter().any(|&branch| err.contains(branch)),
            "PROBLEM: Error missing fallback context\n\
             CONTEXT: Oracle - Standards (error message quality)\n\
             EXPECTED: Error mentions attempted branches: {attempted:?}\n\
             ACTUAL: {err}\n\
             FREQUENCY: Always on branch resolution failure"
        );
    }

    /// Oracle: History - Check fallback behavior is predictable
    /// PROOF: Fallback order should be documented and consistent
    pub fn assert_fallback_order<T>(result: &Result<T>, attempted: &[&str]) {
        if let Err(e) = result {
            let err_str = e.to_string();
            // At least mention what was tried
            assert!(
                attempted.iter().any(|&branch| err_str.contains(branch)),
                "PROBLEM: Error doesn't document fallback attempts\n\
                 CONTEXT: Oracle - History (predictable fallback)\n\
                 EXPECTED: Error lists attempted branches: {attempted:?}\n\
                 ACTUAL: {err_str}\n\
                 FREQUENCY: Always on fallback failure"
            );
        }
    }

    /// Oracle: World - File status matches filesystem reality
    /// PROOF: Verifies git status matches actual file state
    pub fn assert_file_status_valid(change_type: &ChangeType, expected_exists: bool) {
        match change_type {
            ChangeType::Added => {
                assert!(
                    expected_exists,
                    "PROBLEM: File marked as Added but doesn't exist\n\
                     CONTEXT: Oracle - World (file existence)\n\
                     EXPECTED: File should exist for Added status\n\
                     ACTUAL: File missing\n\
                     FREQUENCY: Always on filesystem mismatch"
                );
            }
            ChangeType::Deleted => {
                assert!(
                    !expected_exists,
                    "PROBLEM: File marked as Deleted but still exists\n\
                     CONTEXT: Oracle - World (file existence)\n\
                     EXPECTED: File should not exist for Deleted status\n\
                     ACTUAL: File still present\n\
                     FREQUENCY: Always on filesystem mismatch"
                );
            }
            ChangeType::Modified | ChangeType::Renamed { .. } | ChangeType::Copied { .. } => {
                assert!(
                    expected_exists,
                    "PROBLEM: File marked as changed but doesn't exist\n\
                     CONTEXT: Oracle - World (file existence)\n\
                     EXPECTED: File should exist for modification\n\
                     ACTUAL: File missing\n\
                     FREQUENCY: Always on filesystem mismatch"
                );
            }
        }
    }

    /// Oracle: Comparable - Binary detection matches git behavior
    /// PROOF: Binary files should be detected consistently with git
    pub fn assert_binary_detection(diff: &str, is_binary: bool) {
        if is_binary {
            assert!(
                diff.contains("Binary") || diff.is_empty(),
                "PROBLEM: Binary file not detected\n\
                 CONTEXT: Oracle - Comparable (git binary detection)\n\
                 EXPECTED: Diff contains 'Binary' or is empty\n\
                 ACTUAL: {diff}\n\
                 FREQUENCY: Always on binary file"
            );
        } else {
            assert!(
                !diff.contains("Binary"),
                "PROBLEM: Text file marked as binary\n\
                 CONTEXT: Oracle - Comparable (git binary detection)\n\
                 EXPECTED: Diff should not contain 'Binary'\n\
                 ACTUAL: {diff}\n\
                 FREQUENCY: Always on text file"
            );
        }
    }
}

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Git hooks testing utilities (Unix only)
#[cfg(unix)]
#[allow(dead_code)]
pub struct GitHooksTestHelper;

#[cfg(unix)]
#[allow(dead_code)]
impl GitHooksTestHelper {
    /// Create a git hook script
    pub fn create_hook(
        repo_path: &Path,
        hook_name: &str,
        content: &str,
        should_fail: bool,
    ) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let hooks_dir = repo_path.join(".git").join("hooks");
        fs::create_dir_all(&hooks_dir)?;
        let hook_path = hooks_dir.join(hook_name);
        let mut file = File::create(&hook_path)?;
        writeln!(file, "#!/bin/sh")?;
        writeln!(file, "echo \"Running {hook_name} hook\"")?;
        writeln!(file, "{content}")?;
        if should_fail {
            writeln!(file, "exit 1")?;
        } else {
            writeln!(file, "exit 0")?;
        }
        file.flush()?;

        // Make the hook executable
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;

        Ok(())
    }
}

/// Environment helpers for testing
#[allow(dead_code)]
pub struct TestEnvironment;

#[allow(dead_code)]
impl TestEnvironment {
    /// Check if we should skip remote tests
    pub fn should_skip_remote_tests() -> bool {
        std::env::var("CI").is_ok() || std::env::var("SKIP_REMOTE_TESTS").is_ok()
    }

    /// Check if we should skip integration tests
    pub fn should_skip_integration_tests() -> bool {
        std::env::var("SKIP_INTEGRATION_TESTS").is_ok()
    }

    /// Setup for tests that need API keys
    pub fn setup_api_test_env() -> Option<String> {
        std::env::var("GOOGLE_API_KEY").ok()
    }
}

/// Risk-based test categorization markers
/// High Risk: Core operations that can cause data loss or corruption
#[allow(dead_code)]
pub mod risk {
    pub const HIGH: &str = "high_risk";
    pub const MEDIUM: &str = "medium_risk";
    pub const LOW: &str = "low_risk";
}

/// Test oracle categories following FEW HICCUPS heuristic
#[allow(dead_code)]
pub mod oracle {
    /// Familiarity: How you expect it to work based on experience
    pub const FAMILIARITY: &str = "Familiarity oracle";
    /// Explainability: If you can't explain why, it might be a bug
    pub const EXPLAINABILITY: &str = "Explainability oracle";
    /// World: How things work in the real world
    pub const WORLD: &str = "World oracle";
    /// History: How the product used to work (regression detection)
    pub const HISTORY: &str = "History oracle";
    /// Image: Brand, style, reputation
    pub const IMAGE: &str = "Image oracle";
    /// Comparable Products: How similar products work
    pub const COMPARABLE: &str = "Comparable Products oracle";
    /// Claims: What documentation says
    pub const CLAIMS: &str = "Claims oracle";
    /// User Expectations: What users find intuitive
    pub const USER_EXPECTATIONS: &str = "User Expectations oracle";
    /// Purpose: Stated or implied goals
    pub const PURPOSE: &str = "Purpose oracle";
    /// Standards: Industry or regulatory standards
    pub const STANDARDS: &str = "Standards oracle";
}

/// Complex repository builder for exploratory testing
/// PROOF: Enables testing edge cases and recovery scenarios
#[allow(dead_code)]
#[allow(clippy::struct_excessive_bools)]
pub struct ComplexRepoBuilder {
    num_commits: usize,
    num_branches: usize,
    has_tags: bool,
    has_remote: bool,
    include_binary_files: bool,
    special_characters: bool,
    corrupt_config: bool,
}

#[allow(dead_code)]
impl ComplexRepoBuilder {
    pub fn new() -> Self {
        Self {
            num_commits: 1,
            num_branches: 0,
            has_tags: false,
            has_remote: false,
            include_binary_files: false,
            special_characters: false,
            corrupt_config: false,
        }
    }

    #[must_use]
    pub fn with_commits(mut self, n: usize) -> Self {
        self.num_commits = n;
        self
    }

    #[must_use]
    pub fn with_branches(mut self, n: usize) -> Self {
        self.num_branches = n;
        self
    }

    #[must_use]
    pub fn with_tags(mut self) -> Self {
        self.has_tags = true;
        self
    }

    #[must_use]
    pub fn with_remote(mut self) -> Self {
        self.has_remote = true;
        self
    }

    #[must_use]
    pub fn with_binary_files(mut self) -> Self {
        self.include_binary_files = true;
        self
    }

    #[must_use]
    pub fn with_special_characters(mut self) -> Self {
        self.special_characters = true;
        self
    }

    #[must_use]
    pub fn with_corrupted_config(mut self) -> Self {
        self.corrupt_config = true;
        self
    }

    pub fn build(self) -> Result<(TempDir, GitRepo), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path())?;

        let mut config = repo.config()?;
        config.set_str("user.name", "Test User")?;
        config.set_str("user.email", "test@example.com")?;

        let signature = git2::Signature::now("Test User", "test@example.com")?;

        // Create initial commit
        let mut index = repo.index()?;
        fs::write(temp_dir.path().join("README.md"), "# Test Project")?;
        index.add_path(Path::new("README.md"))?;
        index.write()?;

        let mut parent_commit: Option<git2::Commit> = None;

        for i in 0..self.num_commits {
            let file_path = temp_dir.path().join(format!("file_{i}.txt"));
            fs::write(&file_path, format!("Content {i}"))?;
            index.add_path(Path::new(format!("file_{i}.txt").as_str()))?;
            index.write()?;

            let tree_id = index.write_tree()?;
            let tree = repo.find_tree(tree_id)?;

            let parents: Vec<&git2::Commit> = parent_commit.iter().collect();
            let commit = repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                &format!("Commit {i}"),
                &tree,
                &parents,
            )?;
            parent_commit = Some(repo.find_commit(commit)?);
        }

        // Add branches
        if let Some(ref parent) = parent_commit {
            for i in 0..self.num_branches {
                repo.branch(&format!("branch_{i}"), parent, false)?;
            }
        }

        // Add tags
        if self.has_tags
            && let Some(ref parent) = parent_commit
        {
            let parent_oid = parent.id();
            let obj = repo.find_object(parent_oid, None)?;
            repo.tag("v1.0.0", &obj, &signature, "Version 1.0.0", false)?;
        }

        // Add binary files
        if self.include_binary_files {
            let binary = MockDataBuilder::mock_binary_content();
            fs::write(temp_dir.path().join("image.png"), binary)?;
            index.add_path(Path::new("image.png"))?;
            index.write()?;
        }

        // Add files with special characters
        if self.special_characters {
            let special_dir = temp_dir.path().join("特殊字符");
            fs::create_dir_all(&special_dir)?;
            fs::write(special_dir.join("file with spaces.txt"), "content")?;
            fs::write(special_dir.join("file-with-dashes.txt"), "content")?;
            index.add_path(Path::new("特殊字符/file with spaces.txt").as_ref())?;
            index.add_path(Path::new("特殊字符/file-with-dashes.txt").as_ref())?;
            index.write()?;
        }

        // Corrupt config if requested
        if self.corrupt_config {
            let git_dir = temp_dir.path().join(".git");
            let config_path = git_dir.join("config");
            if config_path.exists() {
                let corrupted = fs::read_to_string(&config_path)?;
                fs::write(&config_path, corrupted.replace("true", "INVALID"))?;
            }
        }

        let git_repo = GitRepo::new(temp_dir.path())?;
        Ok((temp_dir, git_repo))
    }
}

impl Default for ComplexRepoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for testing error conditions
#[allow(dead_code)]
pub struct ErrorScenarioBuilder {
    scenario: String,
}

#[allow(dead_code)]
impl ErrorScenarioBuilder {
    pub fn invalid_repo_path() -> Self {
        Self {
            scenario: "invalid_repo_path".to_string(),
        }
    }

    pub fn corrupted_index() -> Self {
        Self {
            scenario: "corrupted_index".to_string(),
        }
    }

    pub fn missing_user_config() -> Self {
        Self {
            scenario: "missing_user_config".to_string(),
        }
    }

    pub fn build(self) -> Result<GitRepo, Box<dyn std::error::Error>> {
        match self.scenario.as_str() {
            "invalid_repo_path" => {
                let temp_dir = TempDir::new()?;
                // Don't initialize repo - path doesn't exist
                GitRepo::new(temp_dir.path()).map_err(std::convert::Into::into)
            }
            "missing_user_config" => {
                let temp_dir = TempDir::new()?;
                let _repo = Repository::init(temp_dir.path())?;
                // Don't set user config
                GitRepo::new(temp_dir.path()).map_err(std::convert::Into::into)
            }
            _ => Err("Unknown error scenario".into()),
        }
    }
}

/// Session-based test management (SBTM) test session info
/// PROOF: Documents testing mission and findings
#[allow(dead_code)]
pub struct TestSession {
    pub charter: String,
    pub mission: String,
    pub findings: Vec<String>,
    pub risks_identified: Vec<String>,
}

#[allow(dead_code)]
impl TestSession {
    pub fn new(charter: &str, mission: &str) -> Self {
        Self {
            charter: charter.to_string(),
            mission: mission.to_string(),
            findings: Vec::new(),
            risks_identified: Vec::new(),
        }
    }

    pub fn add_finding(&mut self, finding: &str) {
        self.findings.push(finding.to_string());
    }

    pub fn add_risk(&mut self, risk: &str) {
        self.risks_identified.push(risk.to_string());
    }
}
