use crate::core::context::{ChangeType, RecentCommit, StagedFile};
use crate::git::utils::is_binary_diff;
use anyhow::{Context, Result};
use git2::{DiffOptions, Repository, StatusOptions};
use log::debug;
use std::fs;
use std::path::Path;

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
pub fn get_file_statuses(repo: &Repository) -> Result<Vec<StagedFile>> {
    debug!("Getting file statuses");
    let mut staged_files = Vec::new();

    // Peel HEAD tree once
    let head_tree = match repo.head() {
        Ok(head) => Some(head.peel_to_commit()?.tree()?),
        Err(_) => None,
    };

    // Get all staged changes in one diff operation (much faster than file-by-file)
    let mut diff_options = DiffOptions::new();
    let diff = repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut diff_options))?;

    // Use libgit2's built-in rename detection
    let mut find_options = git2::DiffFindOptions::new();
    find_options.renames(true);
    find_options.copies(true);
    find_options.remove_unmodified(true);
    let mut diff = diff; // Make it mutable to detect renames
    diff.find_similar(Some(&mut find_options))?;

    for (i, delta) in diff.deltas().enumerate() {
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .and_then(|p| p.to_str())
            .context("Could not get path")?;

        let change_type = match delta.status() {
            git2::Delta::Added => ChangeType::Added,
            git2::Delta::Modified => ChangeType::Modified,
            git2::Delta::Deleted => ChangeType::Deleted,
            git2::Delta::Renamed => {
                let from = delta
                    .old_file()
                    .path()
                    .and_then(|p| p.to_str())
                    .unwrap_or_default()
                    .to_string();
                ChangeType::Renamed {
                    from,
                    similarity: 0,
                }
            }
            git2::Delta::Copied => {
                let from = delta
                    .old_file()
                    .path()
                    .and_then(|p| p.to_str())
                    .unwrap_or_default()
                    .to_string();
                ChangeType::Copied {
                    from,
                    similarity: 0,
                }
            }
            _ => continue,
        };

        let should_exclude = repo.is_path_ignored(path).unwrap_or(false);

        let diff_text = if should_exclude {
            String::from("[Content excluded]")
        } else {
            // Create patch for this delta
            let mut file_patch = git2::Patch::from_diff(&diff, i)?
                .ok_or_else(|| anyhow::anyhow!("Failed to get patch for {}", path))?;

            let buf = file_patch.to_buf()?;
            let text = String::from_utf8_lossy(&buf).to_string();
            if is_binary_diff(&text) {
                String::from("[Binary file changed]")
            } else {
                text
            }
        };

        let content = if should_exclude
            || !matches!(change_type, ChangeType::Modified)
            || is_binary_diff(&diff_text)
        {
            None
        } else {
            let path_obj = Path::new(path);
            if path_obj.exists() {
                fs::read_to_string(path_obj).ok()
            } else {
                None
            }
        };

        staged_files.push(StagedFile {
            path: path.to_string(),
            change_type,
            diff: diff_text,
            content,
            content_excluded: should_exclude,
        });
    }

    debug!("Found {} staged files", staged_files.len());
    Ok(staged_files)
}

/// Gets unstaged file changes from the repository
///
/// # Returns
///
/// A Result containing a Vec of `StagedFile` objects for unstaged changes or an error.
pub fn get_unstaged_file_statuses(repo: &Repository) -> Result<Vec<StagedFile>> {
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

            let should_exclude = repo.is_path_ignored(path).unwrap_or(false);
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

            unstaged_files.push(StagedFile {
                path: path.to_string(),
                change_type,
                diff,
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
