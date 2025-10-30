use crate::analyzer;
use crate::analyzer::GitIgnoreMatcher;
use crate::core::context::{ChangeType, RecentCommit, StagedFile};
use crate::git::utils::is_binary_diff;
use anyhow::{Context, Result};
use git2::{DiffOptions, Repository, StatusOptions};
use std::fs;
use std::path::Path;
use log::debug;

/// Collects repository information about files and branches
#[derive(Debug)]
pub struct RepoFilesInfo {
    pub branch: String,
    pub recent_commits: Vec<RecentCommit>,
    pub staged_files: Vec<StagedFile>,
    pub file_paths: Vec<String>,
}

/// Retrieves the status of files in the repository.
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `gitignore_matcher` - The gitignore matcher for file exclusion
///
/// # Returns
///
/// A Result containing a Vec of `StagedFile` objects or an error.
pub fn get_file_statuses(repo: &Repository, gitignore_matcher: &GitIgnoreMatcher) -> Result<Vec<StagedFile>> {
    debug!("Getting file statuses");
    let mut staged_files = Vec::new();

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut opts))?;

    for entry in statuses.iter() {
        let path = entry.path().context("Could not get path")?;
        let status = entry.status();

        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            let change_type = if status.is_index_new() {
                ChangeType::Added
             } else if status.is_index_modified() {
                 ChangeType::Modified
             } else {
                 ChangeType::Deleted
             };

             let should_exclude = gitignore_matcher.should_exclude(path);
             let diff = if should_exclude {
                 String::from("[Content excluded]")
             } else {
                 get_diff_for_file(repo, path)?
             };

            let content =
                if should_exclude || change_type != ChangeType::Modified || is_binary_diff(&diff) {
                    None
                } else {
                    let path_obj = Path::new(path);
                    if path_obj.exists() {
                        Some(fs::read_to_string(path_obj)?)
                    } else {
                        None
                    }
                };

            let analyzer = analyzer::get_analyzer(path);
            let staged_file = StagedFile {
                path: path.to_string(),
                change_type: change_type.clone(),
                diff: diff.clone(),
                analysis: Vec::new(),
                content: content.clone(),
                content_excluded: should_exclude,
            };

            let analysis = if should_exclude {
                vec!["[Analysis excluded]".to_string()]
            } else {
                analyzer.analyze(path, &staged_file)
            };

            staged_files.push(StagedFile {
                path: path.to_string(),
                change_type,
                diff,
                analysis,
                content,
                content_excluded: should_exclude,
            });
        }
    }

    debug!("Found {} staged files", staged_files.len());
    Ok(staged_files)
}

/// Retrieves the diff for a specific file.
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `path` - The path of the file to get the diff for.
///
/// # Returns
///
/// A Result containing the diff as a String or an error.
pub fn get_diff_for_file(repo: &Repository, path: &str) -> Result<String> {
    debug!("Getting diff for file: {}", path);
    let mut diff_options = DiffOptions::new();
    diff_options.pathspec(path);

    let tree = match repo.head() {
        Ok(head) => Some(head.peel_to_tree()?),
        Err(_) => {
            // For fresh repos with no HEAD, treat as if comparing against empty tree
            None
        }
    };

    let diff = repo.diff_tree_to_workdir_with_index(tree.as_ref(), Some(&mut diff_options))?;

    let mut diff_string = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = match line.origin() {
            '+' | '-' | ' ' => line.origin(),
            _ => ' ',
        };
        diff_string.push(origin);
        diff_string.push_str(&String::from_utf8_lossy(line.content()));
        true
    })?;

    if is_binary_diff(&diff_string) {
        Ok("[Binary file changed]".to_string())
    } else {
        debug!("Generated diff for {} ({} bytes)", path, diff_string.len());
        Ok(diff_string)
    }
}

/// Gets unstaged file changes from the repository
///
/// # Returns
///
/// A Result containing a Vec of `StagedFile` objects for unstaged changes or an error.
pub fn get_unstaged_file_statuses(repo: &Repository, gitignore_matcher: &GitIgnoreMatcher) -> Result<Vec<StagedFile>> {
    debug!("Getting unstaged file statuses");
    let mut unstaged_files = Vec::new();

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut opts))?;

    for entry in statuses.iter() {
        let path = entry.path().context("Could not get path")?;
        let status = entry.status();

        // Look for changes in the working directory (unstaged)
        if status.is_wt_new() || status.is_wt_modified() || status.is_wt_deleted() {
            let change_type = if status.is_wt_new() {
                ChangeType::Added
            } else if status.is_wt_modified() {
                ChangeType::Modified
             } else {
                 ChangeType::Deleted
             };

             let should_exclude = gitignore_matcher.should_exclude(path);
             let diff = if should_exclude {
                String::from("[Content excluded]")
            } else {
                get_diff_for_unstaged_file(repo, path)?
            };

            let content =
                if should_exclude || change_type != ChangeType::Modified || is_binary_diff(&diff) {
                    None
                } else {
                    let path_obj = Path::new(path);
                    if path_obj.exists() {
                        Some(fs::read_to_string(path_obj)?)
                    } else {
                        None
                    }
                };

            let analyzer = analyzer::get_analyzer(path);
            let unstaged_file = StagedFile {
                path: path.to_string(),
                change_type: change_type.clone(),
                diff: diff.clone(),
                analysis: Vec::new(),
                content: content.clone(),
                content_excluded: should_exclude,
            };

            let analysis = if should_exclude {
                vec!["[Analysis excluded]".to_string()]
            } else {
                analyzer.analyze(path, &unstaged_file)
            };

            unstaged_files.push(StagedFile {
                path: path.to_string(),
                change_type,
                diff,
                analysis,
                content,
                content_excluded: should_exclude,
            });
        }
    }

    debug!("Found {} unstaged files", unstaged_files.len());
    Ok(unstaged_files)
}

/// Gets the diff for an unstaged file
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `path` - The path of the file to get the diff for.
///
/// # Returns
///
/// A Result containing the diff as a String or an error.
pub fn get_diff_for_unstaged_file(repo: &Repository, path: &str) -> Result<String> {
    debug!("Getting unstaged diff for file: {}", path);
    let mut diff_options = DiffOptions::new();
    diff_options.pathspec(path);

    // For unstaged changes, we compare the index (staged) to the working directory
    let diff = repo.diff_index_to_workdir(None, Some(&mut diff_options))?;

    let mut diff_string = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = match line.origin() {
            '+' | '-' | ' ' => line.origin(),
            _ => ' ',
        };
        diff_string.push(origin);
        diff_string.push_str(&String::from_utf8_lossy(line.content()));
        true
    })?;

    if is_binary_diff(&diff_string) {
        Ok("[Binary file changed]".to_string())
    } else {
        debug!(
            "Generated unstaged diff for {} ({} bytes)",
            path,
            diff_string.len()
        );
        Ok(diff_string)
    }
}
