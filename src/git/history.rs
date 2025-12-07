//! Git commit history functionality
//!
//! This module handles retrieving and processing commit history, including:
//! - Recent commits
//! - File-specific commit history
//! - Author commit history

use crate::core::commit_cache::{CachedCommitMessage, CommitMessageCache};
use crate::core::context::RecentCommit;

use anyhow::Result;
use git2::Repository;
use log::debug;
use std::collections::HashSet;
use std::path::Path;

/// Retrieves recent commits from the repository.
///
/// # Arguments
///
/// * `repo` - Reference to an open git2 Repository
/// * `count` - The number of recent commits to retrieve
///
/// # Returns
///
/// A Result containing a Vec of `RecentCommit` objects or an error.
pub fn get_recent_commits(repo: &Repository, count: usize) -> Result<Vec<RecentCommit>> {
    debug!("Fetching {count} recent commits");
    let mut revwalk = repo.revwalk()?;

    // For fresh repos with no commits, push_head() will fail, so return empty vec
    if revwalk.push_head().is_err() {
        debug!("No HEAD found (fresh repository), returning empty recent commits");
        return Ok(Vec::new());
    }

    let commits = revwalk
        .take(count)
        .map(|oid| {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;
            Ok(RecentCommit {
                hash: oid.to_string(),
                message: commit.message().unwrap_or_default().to_string(),
                timestamp: commit.time().seconds().to_string(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    debug!("Retrieved {} recent commits", commits.len());
    Ok(commits)
}

/// Retrieves recent commits that touched any of the specified file paths.
///
/// This is more relevant than generic recent commits because it returns only
/// commits that actually modified the files being changed, similar to
/// `git log --follow -- <path>` but for multiple files.
///
/// # Arguments
///
/// * `repo` - Reference to an open git2 Repository
/// * `file_paths` - The file paths to filter commits by
/// * `max_commits` - Maximum number of commits to return
///
/// # Returns
///
/// A Result containing a Vec of `RecentCommit` objects that touched the files.
pub fn get_commits_for_files(
    repo: &Repository,
    file_paths: &[String],
    max_commits: usize,
) -> Result<Vec<RecentCommit>> {
    debug!(
        "Fetching up to {max_commits} commits for {} files",
        file_paths.len()
    );

    if file_paths.is_empty() {
        debug!("No files specified, returning empty commits");
        return Ok(Vec::new());
    }

    let mut revwalk = repo.revwalk()?;

    // For fresh repos with no commits, push_head() will fail
    if revwalk.push_head().is_err() {
        debug!("No HEAD found (fresh repository), returning empty commits");
        return Ok(Vec::new());
    }

    let mut relevant_commits = Vec::new();
    let file_set: HashSet<&str> = file_paths.iter().map(String::as_str).collect();

    // Process commits until we have enough or run out
    for oid_result in revwalk {
        if relevant_commits.len() >= max_commits {
            break;
        }

        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        if commit_touches_files(repo, &commit, &file_set)? {
            relevant_commits.push(RecentCommit {
                hash: oid.to_string(),
                message: commit.message().unwrap_or_default().to_string(),
                timestamp: commit.time().seconds().to_string(),
            });
        }
    }

    debug!(
        "Found {} commits that touched the specified files",
        relevant_commits.len()
    );
    Ok(relevant_commits)
}

/// Checks if a commit touches any files in the given file set
fn commit_touches_files(
    repo: &Repository,
    commit: &git2::Commit,
    file_set: &HashSet<&str>,
) -> Result<bool> {
    let commit_tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };

    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None)?;

    for delta in diff.deltas() {
        // Check new_file path (for added/modified/renamed-to)
        if let Some(path) = delta.new_file().path() {
            if let Some(path_str) = path.to_str() {
                if file_set.contains(path_str) {
                    return Ok(true);
                }
            }
        }
        // Check old_file path (for deleted/renamed-from)
        if let Some(path) = delta.old_file().path() {
            if let Some(path_str) = path.to_str() {
                if file_set.contains(path_str) {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

/// Retrieves the author's recent commit messages with caching.
///
/// # Arguments
///
/// * `repo` - Reference to an open git2 Repository
/// * `repo_path` - Path to the repository (for caching)
/// * `author_email` - The email of the author to filter by
/// * `count` - The number of recent commits to retrieve
///
/// # Returns
///
/// A Result containing a Vec of commit message strings or an error.
pub fn get_author_commit_history(
    repo: &Repository,
    repo_path: &Path,
    author_email: &str,
    count: usize,
) -> Result<Vec<String>> {
    debug!("Fetching {count} recent commits for author: {author_email}");
    let mut revwalk = repo.revwalk()?;

    // For fresh repos with no commits, push_head() will fail, so return empty vec
    if revwalk.push_head().is_err() {
        debug!("No HEAD found (fresh repository), returning empty author history");
        return Ok(Vec::new());
    }

    let mut cached_messages = Vec::new();
    let mut commit_messages = Vec::new();

    for oid_result in revwalk.take(count) {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;
        let author = commit.author();

        // Filter by author email
        if author.email() == Some(author_email) {
            let message = commit.message().unwrap_or_default().to_string();
            let timestamp = commit.time().seconds().to_string();
            let hash = format!("{oid}");

            // Store for caching
            cached_messages.push(CachedCommitMessage {
                message: message.clone(),
                timestamp,
                hash,
            });

            // Store for return
            commit_messages.push(message);
        }
    }

    // Cache the retrieved messages
    if !cached_messages.is_empty() {
        cache_commit_messages(repo_path, author_email, cached_messages)?;
    }

    debug!(
        "Retrieved {} commits for author {author_email}",
        commit_messages.len()
    );
    Ok(commit_messages)
}

/// Caches commit messages for future use
fn cache_commit_messages(
    repo_path: &Path,
    author_email: &str,
    messages: Vec<CachedCommitMessage>,
) -> Result<()> {
    let mut cache = CommitMessageCache::new()?;
    let repo_path_str = repo_path.to_string_lossy().to_string();
    cache.add_commit_messages(author_email, &repo_path_str, messages);
    cache.save()?;
    Ok(())
}
