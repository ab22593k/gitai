use crate::config::Config;
use crate::core::commit_cache::CommitMessageCache;
use crate::core::context::{CommitContext, RecentCommit, StagedFile};

use crate::git::commit::{self, CommitResult};
use crate::git::files::{RepoFilesInfo, get_file_statuses, get_unstaged_file_statuses};
use crate::git::history;
use crate::git::hooks;
use crate::git::utils::is_inside_work_tree;
use anyhow::{Context as AnyhowContext, Result, anyhow};
use git2::{Repository, Tree};
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use tokio::task;

use log::debug;
use tempfile::TempDir;
use url::Url;

use super::ignore_matcher::GitIgnoreMatcher;

/// Represents a Git repository and provides methods for interacting with it.
pub struct GitRepo {
    repo_path: PathBuf,
    /// Optional temporary directory for cloned repositories
    #[allow(dead_code)] // This field is needed to maintain ownership of temp directories
    temp_dir: Option<TempDir>,
    /// Whether this is a remote repository
    is_remote: bool,
    /// Original remote URL if this is a cloned repository
    remote_url: Option<String>,
    /// `GitIgnore` matcher for file exclusion
    gitignore_matcher: GitIgnoreMatcher,
}

impl GitRepo {
    /// Creates a new `GitRepo` instance from a local path.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - The path to the Git repository.
    ///
    /// # Returns
    ///
    /// A Result containing the `GitRepo` instance or an error.
    pub fn new(repo_path: &Path) -> Result<Self> {
        Ok(Self {
            repo_path: repo_path.to_path_buf(),
            temp_dir: None,
            is_remote: false,
            remote_url: None,
            gitignore_matcher: GitIgnoreMatcher::new(repo_path),
        })
    }

    /// Creates a new `GitRepo` instance, handling both local and remote repositories.
    ///
    /// # Arguments
    ///
    /// * `repository_url` - Optional URL for a remote repository.
    ///
    /// # Returns
    ///
    /// A Result containing the `GitRepo` instance or an error.
    pub fn new_from_url(repository_url: Option<String>) -> Result<Self> {
        if let Some(url) = repository_url {
            Self::clone_remote_repository(&url)
        } else {
            let current_dir = env::current_dir()?;
            Self::new(&current_dir)
        }
    }

    /// Clones a remote repository and creates a `GitRepo` instance for it.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL of the remote repository to clone.
    ///
    /// # Returns
    ///
    /// A Result containing the `GitRepo` instance or an error.
    pub fn clone_remote_repository(url: &str) -> Result<Self> {
        debug!("Cloning remote repository from URL: {url}");

        // Validate URL
        let _ = Url::parse(url).map_err(|e| anyhow!("Invalid repository URL: {e}"))?;

        // Create a temporary directory for the clone
        let temp_dir = TempDir::new()?;
        let temp_path_buf = temp_dir.path().to_path_buf();

        debug!(
            "Created temporary directory for clone: {}",
            temp_path_buf.display()
        );

        // Clone the repository into the temporary directory
        let repo = match Repository::clone(url, &temp_path_buf) {
            Ok(repo) => repo,
            Err(e) => return Err(anyhow!("Failed to clone repository: {e}")),
        };

        debug!(
            "Successfully cloned repository to {}",
            repo.path().display()
        );

        Ok(Self {
            repo_path: temp_path_buf.clone(),
            temp_dir: Some(temp_dir),
            is_remote: true,
            remote_url: Some(url.to_string()),
            gitignore_matcher: GitIgnoreMatcher::new(&temp_path_buf),
        })
    }

    /// Open the repository at the stored path
    pub fn open_repo(&self) -> Result<Repository, git2::Error> {
        Repository::open(&self.repo_path)
    }

    /// Returns whether this `GitRepo` instance is working with a remote repository
    pub fn is_remote(&self) -> bool {
        self.is_remote
    }

    /// Returns the original remote URL if this is a cloned repository
    pub fn get_remote_url(&self) -> Option<&str> {
        self.remote_url.as_deref()
    }

    /// Returns the repository path
    pub fn repo_path(&self) -> &PathBuf {
        &self.repo_path
    }

    /// Updates the remote repository by fetching the latest changes
    pub fn update_remote(&self) -> Result<()> {
        if !self.is_remote {
            return Err(anyhow!("Not a remote repository"));
        }

        debug!("Updating remote repository");
        let repo = self.open_repo()?;

        // Find the default remote (usually "origin")
        let remotes = repo.remotes()?;
        let remote_name = remotes
            .iter()
            .flatten()
            .next()
            .ok_or_else(|| anyhow!("No remote found"))?;

        // Fetch updates from the remote (all branches)
        let mut remote = repo.find_remote(remote_name)?;
        remote.fetch(<&[&str]>::default(), None, None)?;

        debug!("Successfully updated remote repository");
        Ok(())
    }

    /// Retrieves the current branch name.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of recent commits to retrieve.
    ///
    /// # Returns
    ///
    /// A Result containing the branch name as a String or an error.
    pub fn get_current_branch(&self) -> Result<String> {
        let repo = self.open_repo()?;
        if let Ok(head) = repo.head() {
            let branch_name = head.shorthand().unwrap_or("HEAD detached").to_string();
            debug!("Current branch: {branch_name}");
            Ok(branch_name)
        } else {
            // For fresh repos with no commits, default to "main"
            debug!("No HEAD found (fresh repository), defaulting to 'main'");
            Ok("main".to_string())
        }
    }

    /// Executes a Git hook.
    ///
    /// # Arguments
    ///
    /// * `hook_name` - The name of the hook to execute.
    ///
    /// # Returns
    ///
    /// A Result indicating success or an error.
    pub fn execute_hook(&self, hook_name: &str) -> Result<()> {
        let repo = self.open_repo()?;
        hooks::execute_hook(&repo, hook_name, self.is_remote)
    }

    /// Get the root directory of the current git repository
    pub fn get_repo_root() -> Result<PathBuf> {
        // Check if we're in a git repository
        if !is_inside_work_tree()? {
            return Err(anyhow!(
                "Not in a Git repository. Please run this command from within a Git repository."
            ));
        }

        // Use git2 to find the repository root
        let repo = Repository::discover(".").context("Failed to discover git repository")?;
        let workdir = repo
            .workdir()
            .context("Repository has no working directory")?;
        Ok(workdir.to_path_buf())
    }

    /// Retrieves the README content at a specific commit.
    ///
    /// # Arguments
    ///
    /// * `commit_ish` - A string that resolves to a commit.
    ///
    /// # Returns
    ///
    /// A Result containing an `Option<String>` with the README content or an error.
    pub fn get_readme_at_commit(&self, commit_ish: &str) -> Result<Option<String>> {
        let repo = self.open_repo()?;
        let obj = repo.revparse_single(commit_ish)?;
        let tree = obj.peel_to_tree()?;

        Self::find_readme_in_tree(&repo, &tree)
            .context("Failed to find and read README at specified commit")
    }

    /// Finds a README file in the given tree.
    ///
    /// # Arguments
    ///
    /// * `tree` - A reference to a Git tree.
    ///
    /// # Returns
    ///
    /// A Result containing an `Option<String>` with the README content or an error.
    fn find_readme_in_tree(repo: &Repository, tree: &Tree) -> Result<Option<String>> {
        debug!("Searching for README file in the repository");

        let readme_patterns = [
            "README.md",
            "README.markdown",
            "README.txt",
            "README",
            "Readme.md",
            "readme.md",
        ];

        for entry in tree {
            let name = entry.name().unwrap_or("");
            if readme_patterns
                .iter()
                .any(|&pattern| name.eq_ignore_ascii_case(pattern))
            {
                let object = entry.to_object(repo)?;
                if let Some(blob) = object.as_blob()
                    && let Ok(content) = std::str::from_utf8(blob.content())
                {
                    debug!("README file found: {name}");
                    return Ok(Some(content.to_string()));
                }
            }
        }

        debug!("No README file found");
        Ok(None)
    }

    /// Extract files info without crossing async boundaries
    pub fn extract_files_info(&self, include_unstaged: bool) -> Result<RepoFilesInfo> {
        let repo = self.open_repo()?;

        // Get basic repo info
        let branch = self.get_current_branch()?;
        let recent_commits = self.get_recent_commits(5)?;

        // Get staged and unstaged files
        let mut staged_files = get_file_statuses(&repo, &self.gitignore_matcher)?;
        if include_unstaged {
            let unstaged_files = self.get_unstaged_files()?;
            staged_files.extend(unstaged_files);
            debug!("Combined {} files (staged + unstaged)", staged_files.len());
        }

        // Extract file paths for metadata
        let file_paths: Vec<String> = staged_files.iter().map(|file| file.path.clone()).collect();

        Ok(RepoFilesInfo {
            branch,
            recent_commits,
            staged_files,
            file_paths,
        })
    }

    /// Gets unstaged file changes from the repository
    pub fn get_unstaged_files(&self) -> Result<Vec<StagedFile>> {
        let repo = self.open_repo()?;
        get_unstaged_file_statuses(&repo, &self.gitignore_matcher)
    }

    /// Helper method for creating `CommitContext`
    ///
    /// # Arguments
    ///
    /// * `branch` - Branch name
    /// * `recent_commits` - List of recent commits
    /// * `staged_files` - List of staged files
    /// * `project_metadata` - Project metadata
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitContext` or an error.
    fn create_commit_context(
        &self,
        branch: String,
        recent_commits: Vec<RecentCommit>,
        staged_files: Vec<StagedFile>,
    ) -> Result<CommitContext> {
        // Get user info
        let repo = self.open_repo()?;
        let user_name = repo.config()?.get_string("user.name").unwrap_or_default();
        let user_email = repo.config()?.get_string("user.email").unwrap_or_default();

        // Get author's commit history (last 10 commits)
        let author_history = self.get_author_commit_history(&user_email, 10)?;

        // Create and return the context
        Ok(CommitContext::new(
            branch,
            recent_commits,
            staged_files,
            user_name,
            user_email,
            author_history,
        ))
    }

    /// Get Git information including unstaged changes
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration object.
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitContext` or an error.
    pub async fn get_git_info(&self, _config: &Config) -> Result<CommitContext> {
        let repo_path = self.repo_path.clone();
        let gitignore_matcher = self.gitignore_matcher.clone();

        task::spawn_blocking(move || {
            let repo = Repository::open(&repo_path)?;
            debug!("Getting git info for repo path: {}", repo.path().display());

            let branch = Self::get_current_branch_sync(&repo)?;
            let staged_files = get_file_statuses(&repo, &gitignore_matcher)?;

            let file_paths: Vec<String> = staged_files.iter().map(|f| f.path.clone()).collect();
            let recent_commits = if file_paths.is_empty() {
                Self::get_recent_commits_sync(&repo, 10)?
            } else {
                let file_commits = Self::get_commits_for_files_sync(&repo, &file_paths, 10)?;
                if file_commits.is_empty() {
                    Self::get_recent_commits_sync(&repo, 10)?
                } else {
                    file_commits
                }
            };

            let mut context = Self::create_commit_context_sync(
                &repo,
                &repo_path,
                branch,
                recent_commits,
                staged_files,
            )?;

            Self::enhance_context_with_cache_sync(&repo_path, &mut context)?;

            Ok(context)
        })
        .await?
    }

    #[allow(clippy::unnecessary_wraps)]
    fn get_current_branch_sync(repo: &Repository) -> Result<String> {
        if let Ok(head) = repo.head() {
            let branch_name = head.shorthand().unwrap_or("HEAD detached").to_string();
            debug!("Current branch: {branch_name}");
            Ok(branch_name)
        } else {
            debug!("No HEAD found (fresh repository), defaulting to 'main'");
            Ok("main".to_string())
        }
    }

    fn get_recent_commits_sync(repo: &Repository, count: usize) -> Result<Vec<RecentCommit>> {
        history::get_recent_commits(repo, count)
    }

    fn get_commits_for_files_sync(
        repo: &Repository,
        file_paths: &[String],
        max_commits: usize,
    ) -> Result<Vec<RecentCommit>> {
        history::get_commits_for_files(repo, file_paths, max_commits)
    }

    fn create_commit_context_sync(
        repo: &Repository,
        repo_path: &Path,
        branch: String,
        recent_commits: Vec<RecentCommit>,
        staged_files: Vec<StagedFile>,
    ) -> Result<CommitContext> {
        let user_name = repo.config()?.get_string("user.name").unwrap_or_default();
        let user_email = repo.config()?.get_string("user.email").unwrap_or_default();
        let author_history = history::get_author_commit_history(repo, repo_path, &user_email, 10)?;

        Ok(CommitContext::new(
            branch,
            recent_commits,
            staged_files,
            user_name,
            user_email,
            author_history,
        ))
    }

    fn enhance_context_with_cache_sync(
        repo_path: &Path,
        context: &mut CommitContext,
    ) -> Result<()> {
        let cache = CommitMessageCache::new()?;
        let cached_messages =
            cache.get_commit_messages(&context.user_email, &repo_path.to_string_lossy());

        let cached_history: Vec<String> =
            cached_messages.into_iter().map(|msg| msg.message).collect();

        let mut enhanced_history = context.author_history.clone();
        enhanced_history.extend(cached_history);

        let mut unique_history = Vec::new();
        let mut seen = HashSet::new();
        for msg in enhanced_history {
            if seen.insert(msg.clone()) {
                unique_history.push(msg);
            }
        }

        if unique_history.len() > 100 {
            unique_history = unique_history.into_iter().take(100).collect();
        }

        context.author_history = unique_history;
        Ok(())
    }

    /// Get Git information including unstaged changes
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration object.
    /// * `include_unstaged` - Whether to include unstaged changes.
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitContext` or an error.
    pub async fn get_git_info_with_unstaged(
        &self,
        _config: &Config,
        include_unstaged: bool,
    ) -> Result<CommitContext> {
        let repo_path = self.repo_path.clone();
        let gitignore_matcher = self.gitignore_matcher.clone();

        task::spawn_blocking(move || {
            let repo = Repository::open(&repo_path)?;
            debug!(
                "Getting git info for repo path: {}, include_unstaged: {}",
                repo.path().display(),
                include_unstaged
            );

            let branch = Self::get_current_branch_sync(&repo)?;
            let mut staged_files = get_file_statuses(&repo, &gitignore_matcher)?;

            if include_unstaged {
                let unstaged_files = get_unstaged_file_statuses(&repo, &gitignore_matcher)?;
                staged_files.extend(unstaged_files);
                debug!("Combined {} files (staged + unstaged)", staged_files.len());
            }

            let file_paths: Vec<String> = staged_files.iter().map(|f| f.path.clone()).collect();
            let recent_commits = if file_paths.is_empty() {
                Self::get_recent_commits_sync(&repo, 10)?
            } else {
                let file_commits = Self::get_commits_for_files_sync(&repo, &file_paths, 10)?;
                if file_commits.is_empty() {
                    Self::get_recent_commits_sync(&repo, 10)?
                } else {
                    file_commits
                }
            };

            let context = Self::create_commit_context_sync(
                &repo,
                &repo_path,
                branch,
                recent_commits,
                staged_files,
            )?;

            Ok(context)
        })
        .await?
    }

    /// Get Git information for comparing two branches
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration object
    /// * `base_branch` - The base branch (e.g., "main")
    /// * `target_branch` - The target branch (e.g., "feature-branch")
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitContext` for the branch comparison or an error.
    pub fn get_git_info_for_branch_diff(
        &self,
        _config: &Config,
        base_branch: &str,
        target_branch: &str,
    ) -> Result<CommitContext> {
        debug!("Getting git info for branch diff: {base_branch} -> {target_branch}");
        let repo = self.open_repo()?;

        // Extract branch diff info
        let (display_branch, recent_commits, _) = commit::extract_branch_diff_info(
            &repo,
            base_branch,
            target_branch,
            &self.gitignore_matcher,
        )?;

        // Get the actual file changes
        let branch_files = commit::get_branch_diff_files(
            &repo,
            base_branch,
            target_branch,
            &self.gitignore_matcher,
        )?;

        // Create and return the context
        self.create_commit_context(display_branch, recent_commits, branch_files)
    }

    /// Get Git information for a commit range (for PR descriptions)
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration object
    /// * `from` - The starting Git reference (exclusive)
    /// * `to` - The ending Git reference (inclusive)
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitContext` for the commit range or an error.
    pub fn get_git_info_for_commit_range(
        &self,
        _config: &Config,
        from: &str,
        to: &str,
    ) -> Result<CommitContext> {
        debug!("Getting git info for commit range: {from} -> {to}");
        let repo = self.open_repo()?;

        // Extract commit range info
        let (display_range, recent_commits, _) =
            commit::extract_commit_range_info(&repo, from, to, &self.gitignore_matcher)?;

        // Get the actual file changes
        let range_files = commit::get_commit_range_files(&repo, from, to, &self.gitignore_matcher)?;

        // Create and return the context
        self.create_commit_context(display_range, recent_commits, range_files)
    }

    /// Get commits for PR description between two references
    pub fn get_commits_for_pr(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let repo = self.open_repo()?;
        commit::get_commits_for_pr(&repo, from, to)
    }

    /// Get files changed in a commit range
    pub fn get_commit_range_files(&self, from: &str, to: &str) -> Result<Vec<StagedFile>> {
        let repo = self.open_repo()?;
        commit::get_commit_range_files(&repo, from, to, &self.gitignore_matcher)
    }

    /// Retrieves recent commits.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of recent commits to retrieve.
    ///
    /// # Returns
    ///
    /// A Result containing a Vec of `RecentCommit` objects or an error.
    pub fn get_recent_commits(&self, count: usize) -> Result<Vec<RecentCommit>> {
        let repo = self.open_repo()?;
        history::get_recent_commits(&repo, count)
    }

    /// Retrieves recent commits that touched any of the specified file paths.
    ///
    /// This is more relevant than generic recent commits because it returns only
    /// commits that actually modified the files being changed, similar to
    /// `git log --follow -- <path>` but for multiple files.
    ///
    /// # Arguments
    ///
    /// * `file_paths` - The file paths to filter commits by.
    /// * `max_commits` - Maximum number of commits to return.
    ///
    /// # Returns
    ///
    /// A Result containing a Vec of `RecentCommit` objects that touched the files.
    pub fn get_commits_for_files(
        &self,
        file_paths: &[String],
        max_commits: usize,
    ) -> Result<Vec<RecentCommit>> {
        let repo = self.open_repo()?;
        history::get_commits_for_files(&repo, file_paths, max_commits)
    }

    /// Retrieves the author's recent commit messages.
    ///
    /// # Arguments
    ///
    /// * `author_email` - The email of the author to filter by.
    /// * `count` - The number of recent commits to retrieve.
    ///
    /// # Returns
    ///
    /// A Result containing a Vec of commit message strings or an error.
    pub fn get_author_commit_history(
        &self,
        author_email: &str,
        count: usize,
    ) -> Result<Vec<String>> {
        let repo = self.open_repo()?;
        history::get_author_commit_history(&repo, &self.repo_path, author_email, count)
    }

    /// Commits changes and verifies the commit.
    ///
    /// # Arguments
    ///
    /// * `message` - The commit message.
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitResult` or an error.
    pub fn commit_and_verify(&self, message: &str) -> Result<CommitResult> {
        if self.is_remote {
            return Err(anyhow!(
                "Cannot commit to a remote repository in read-only mode"
            ));
        }

        let repo = self.open_repo()?;
        match commit::commit(&repo, message, self.is_remote) {
            Ok(result) => {
                if let Err(e) = self.execute_hook("post-commit") {
                    debug!("Post-commit hook failed: {e}");
                }
                Ok(result)
            }
            Err(e) => {
                debug!("Commit failed: {e}");
                Err(e)
            }
        }
    }

    /// Get Git information for a specific commit
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration object
    /// * `commit_id` - The ID of the commit to analyze
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitContext` or an error.
    pub fn get_git_info_for_commit(
        &self,
        _config: &Config,
        commit_id: &str,
    ) -> Result<CommitContext> {
        debug!("Getting git info for commit: {commit_id}");
        let repo = self.open_repo()?;

        // Get branch name
        let branch = self.get_current_branch()?;

        // Extract commit info
        let commit_info = commit::extract_commit_info(&repo, commit_id, &branch)?;

        // Get the files from commit after async boundary
        let commit_files = commit::get_commit_files(&repo, commit_id, &self.gitignore_matcher)?;

        // Create and return the context
        self.create_commit_context(commit_info.branch, vec![commit_info.commit], commit_files)
    }

    /// Get the commit date for a reference
    pub fn get_commit_date(&self, commit_ish: &str) -> Result<String> {
        let repo = self.open_repo()?;
        commit::get_commit_date(&repo, commit_ish)
    }

    /// Get commits between two references with a callback
    pub fn get_commits_between_with_callback<T, F>(
        &self,
        from: &str,
        to: &str,
        callback: F,
    ) -> Result<Vec<T>>
    where
        F: FnMut(&RecentCommit) -> Result<T>,
    {
        let repo = self.open_repo()?;
        commit::get_commits_between_with_callback(&repo, from, to, callback)
    }

    /// Stream commits between two references with a callback
    pub fn get_commits_between_stream<F>(&self, from: &str, to: &str, callback: F) -> Result<()>
    where
        F: FnMut(&RecentCommit) -> Result<()>,
    {
        let repo = self.open_repo()?;
        commit::get_commits_between_stream(&repo, from, to, callback)
    }

    /// Commit changes to the repository
    pub fn commit(&self, message: &str) -> Result<CommitResult> {
        let repo = self.open_repo()?;
        commit::commit(&repo, message, self.is_remote)
    }

    /// Amend a commit with a new message
    pub fn amend_commit(&self, message: &str, commit_ref: &str) -> Result<CommitResult> {
        let repo = self.open_repo()?;
        commit::amend_commit(&repo, message, commit_ref, self.is_remote)
    }

    /// Check if inside a working tree
    pub fn is_inside_work_tree() -> Result<bool> {
        is_inside_work_tree()
    }

    /// Get the files changed in a specific commit
    pub fn get_commit_files(&self, commit_id: &str) -> Result<Vec<StagedFile>> {
        let repo = self.open_repo()?;
        commit::get_commit_files(&repo, commit_id, &self.gitignore_matcher)
    }

    /// Get just the file paths for a specific commit
    pub fn get_file_paths_for_commit(&self, commit_id: &str) -> Result<Vec<String>> {
        let repo = self.open_repo()?;
        commit::get_file_paths_for_commit(&repo, commit_id)
    }

    /// Get the latest tag that is an ancestor of HEAD
    ///
    /// Uses git describe to find the most recent tag that is an ancestor
    /// of the current HEAD.
    ///
    /// # Returns
    ///
    /// A Result containing an Option with the tag name if found, or None if no tags exist.
    pub fn get_latest_tag(&self) -> Result<Option<String>> {
        let repo = self.open_repo()?;

        // First, check if there are any tags
        let tag_names = repo.tag_names(None::<&str>)?;
        if tag_names.is_empty() {
            return Ok(None);
        }

        let mut opts = git2::DescribeOptions::new();
        opts.describe_tags();
        opts.show_commit_oid_as_fallback(false);

        let describe = match repo.describe(&opts) {
            Ok(d) => d,
            Err(e) => {
                if e.code() == git2::ErrorCode::NotFound {
                    return Ok(None);
                }
                return Err(anyhow!("Failed to describe: {e}"));
            }
        };

        let mut format_opts = git2::DescribeFormatOptions::new();
        format_opts.abbreviated_size(0);

        let tag_name = describe.format(Some(&format_opts))?;

        // If the output looks like a commit hash (40 hex chars), it means no tag was found
        if tag_name.len() == 40 && tag_name.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(None);
        }

        Ok(Some(tag_name))
    }

    /// Get the first (oldest) commit in the repository
    ///
    /// Uses a reverse revision walk to find the oldest commit.
    ///
    /// # Returns
    ///
    /// A Result containing the commit hash as a String.
    pub fn get_first_commit(&self) -> Result<String> {
        let repo = self.open_repo()?;
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;

        // Collect all commits and find the oldest
        let mut commits: Vec<_> = revwalk.filter_map(std::result::Result::ok).collect();
        commits.reverse();

        if let Some(first_oid) = commits.first() {
            Ok(first_oid.to_string())
        } else {
            Err(anyhow!("No commits found in repository"))
        }
    }
}

impl Drop for GitRepo {
    fn drop(&mut self) {
        // The TempDir will be automatically cleaned up when dropped
        if self.is_remote {
            debug!(
                "Cleaning up temporary repository at {}",
                self.repo_path.display()
            );
        }
    }
}
