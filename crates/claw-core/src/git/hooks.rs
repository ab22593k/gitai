//! Git hook execution functionality
//!
//! This module handles the execution of Git hooks (pre-commit, post-commit, etc.)

use anyhow::{Context as AnyhowContext, Result, anyhow};
use git2::Repository;
use log::debug;
use std::path::Path;
use std::process::{Command, Stdio};

/// Executes a Git hook from the given repository.
///
/// # Arguments
///
/// * `repo` - Reference to an open git2 Repository
/// * `repo_path` - Path to the repository
/// * `hook_name` - The name of the hook to execute (e.g., "pre-commit", "post-commit")
/// * `is_remote` - Whether this is a remote repository (hooks are skipped for remote repos)
///
/// # Returns
///
/// A Result indicating success or an error if the hook fails.
pub fn execute_hook(repo: &Repository, hook_name: &str, is_remote: bool) -> Result<()> {
    if is_remote {
        debug!("Skipping hook execution for remote repository");
        return Ok(());
    }

    let hook_path = repo.path().join("hooks").join(hook_name);

    if hook_path.exists() {
        execute_hook_file(&hook_path, repo, hook_name)
    } else {
        debug!("Hook '{hook_name}' not found at {}", hook_path.display());
        Ok(())
    }
}

/// Executes a hook file
fn execute_hook_file(hook_path: &Path, repo: &Repository, hook_name: &str) -> Result<()> {
    debug!("Executing hook: {hook_name}");
    debug!("Hook path: {}", hook_path.display());

    // Get the repository's working directory (top level)
    let repo_workdir = repo
        .workdir()
        .context("Repository has no working directory")?;
    debug!("Repository working directory: {}", repo_workdir.display());

    // Create a command with the proper environment and working directory
    let mut command = Command::new(hook_path);
    command
        .current_dir(repo_workdir) // Use the repository's working directory, not .git
        .env("GIT_DIR", repo.path()) // Set GIT_DIR to the .git directory
        .env("GIT_WORK_TREE", repo_workdir) // Set GIT_WORK_TREE to the working directory
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    debug!("Executing hook command: {command:?}");

    let mut child = command.spawn()?;

    let stdout = child.stdout.take().context("Could not get stdout")?;
    let stderr = child.stderr.take().context("Could not get stderr")?;

    // Stream output in separate threads
    std::thread::spawn(move || {
        std::io::copy(&mut std::io::BufReader::new(stdout), &mut std::io::stdout())
            .expect("Failed to copy data to stdout");
    });
    std::thread::spawn(move || {
        std::io::copy(&mut std::io::BufReader::new(stderr), &mut std::io::stderr())
            .expect("Failed to copy data to stderr");
    });

    let status = child.wait()?;

    if status.success() {
        debug!("Hook '{hook_name}' executed successfully");
        Ok(())
    } else {
        Err(anyhow!(
            "Hook '{}' failed with exit code: {:?}",
            hook_name,
            status.code()
        ))
    }
}
