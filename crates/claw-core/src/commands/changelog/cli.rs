use super::change_log::ChangelogGenerator;
use super::releasenotes::ReleaseNotesGenerator;
use crate::common::CommonParams;
use crate::config::Config;
use crate::git::GitRepo;
use crate::output;
use anyhow::{Context, Result, anyhow};
use colored::Colorize;
use std::env;
use std::sync::Arc;

pub struct ChangelogCommandConfig {
    pub from: Option<String>,
    pub to: Option<String>,
    pub repository_url: Option<String>,
    pub update_file: bool,
    pub save: bool,
    pub changelog_path: Option<String>,
    pub version_name: Option<String>,
}

/// Handles the changelog generation command.
///
/// This function orchestrates the process of generating a changelog based on the provided
/// parameters. It sets up the necessary environment, creates a `GitRepo` instance,
/// and delegates the actual generation to the `ChangelogGenerator`.
///
/// # Arguments
///
/// * `common` - Common parameters for the command, including configuration overrides.
/// * `config` - Configuration for the changelog command.
///
/// # Returns
///
/// Returns a Result indicating success or containing an error if the operation failed.
pub async fn handle_changelog_command(
    common: CommonParams,
    config: ChangelogCommandConfig,
) -> Result<()> {
    let ChangelogCommandConfig {
        from,
        to,
        repository_url,
        update_file,
        save,
        changelog_path,
        version_name,
    } = config;

    // Load and apply configuration
    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;

    // Create a spinner to indicate progress
    let mut spinner = output::create_tui_spinner("Generating changelog...");

    // Ensure we're in a git repository
    if let Err(e) = config.check_environment() {
        output::print_error(&format!("Error: {e}"));
        output::print_info("\nPlease ensure the following:");
        output::print_info("1. Git is installed and accessible from the command line.");
        output::print_info(
            "2. You are running this command from within a Git repository or provide a repository URL with --repo.",
        );
        output::print_info("3. You have set up your configuration using 'git config'.");
        return Err(e);
    }

    // Use the repository URL from command line or common params
    let repo_url = repository_url.or(common.repository_url);

    // Create a GitRepo instance based on the URL or current directory
    let git_repo = if let Some(url) = repo_url {
        Arc::new(GitRepo::clone_remote_repository(&url).context("Failed to clone repository")?)
    } else {
        let repo_path = env::current_dir()?;
        Arc::new(GitRepo::new(&repo_path).context("Failed to create GitRepo")?)
    };

    // Keep a clone of the Arc for updating the changelog later if needed
    let git_repo_for_update = Arc::clone(&git_repo);

    // Handle --save flag: auto-detect starting reference and update CHANGELOG.md
    let should_update_file = update_file || save;
    let changelog_file_path = if save {
        // If --save is used, default to CHANGELOG.md in current directory
        changelog_path.or_else(|| Some("CHANGELOG.md".to_string()))
    } else {
        changelog_path
    };

    // Resolve 'from' reference if not provided (for --save or manual use)
    let from_ref = if let Some(f) = from {
        Some(f)
    } else if save {
        // Auto-detect the starting reference for --save
        output::print_info("Detecting latest tag...");
        match git_repo.get_latest_tag() {
            Ok(Some(tag)) => {
                output::print_success(&format!("Found latest tag: {}", tag));
                Some(tag)
            }
            Ok(None) => {
                output::print_info("No tags found, using first commit...");
                match git_repo.get_first_commit() {
                    Ok(commit) => Some(commit),
                    Err(e) => {
                        output::print_error(&format!("Failed to get first commit: {e}"));
                        return Err(anyhow!("Cannot determine starting point for changelog"));
                    }
                }
            }
            Err(e) => {
                output::print_error(&format!("Failed to get latest tag: {e}"));
                return Err(anyhow!("Failed to detect latest tag: {}", e));
            }
        }
    } else {
        None
    };

    // Validate that we have a 'from' reference
    let from_ref = from_ref
        .ok_or_else(|| anyhow!("Starting reference (--from) is required when not using --save"))?;

    // Set the default 'to' reference if not provided
    let to = to.unwrap_or_else(|| "HEAD".to_string());

    let detail_level = common.detail_level;

    // Generate the changelog
    let changelog =
        ChangelogGenerator::generate(git_repo, &from_ref, &to, &config, detail_level).await?;

    // Clear the spinner and display the result
    spinner.tick();

    // Output the changelog with decorative borders
    output::print_bordered_content(&changelog);

    // Update the changelog file if requested
    if should_update_file {
        let path = changelog_file_path.unwrap_or_else(|| "CHANGELOG.md".to_string());
        let mut update_spinner =
            output::create_tui_spinner(&format!("Updating changelog file at {path}..."));

        match ChangelogGenerator::update_changelog_file(
            &changelog,
            &path,
            &git_repo_for_update,
            &to,
            version_name,
        ) {
            Ok(()) => {
                update_spinner.tick();
                output::print_success(&format!(
                    "✨ Changelog successfully updated at {}",
                    path.bright_green()
                ));
            }
            Err(e) => {
                update_spinner.tick();
                output::print_error(&format!("Failed to update changelog file: {e}"));
            }
        }
    }

    Ok(())
}

/// Handles the release notes generation command.
///
/// This function orchestrates the process of generating release notes based on the provided
/// parameters. It sets up the necessary environment, creates a `GitRepo` instance,
/// and delegates the actual generation to the `ReleaseNotesGenerator`.
///
/// # Arguments
///
/// * `common` - Common parameters for the command, including configuration overrides.
/// * `from` - The starting point (commit or tag) for the release notes.
/// * `to` - The ending point for the release notes. Defaults to "HEAD" if not provided.
/// * `repository_url` - Optional URL of the remote repository to use.
/// * `version_name` - Optional version name to use instead of extracting from Git refs.
///
/// # Returns
///
/// Returns a Result indicating success or containing an error if the operation failed.
pub async fn handle_release_notes_command(
    common: CommonParams,
    from: String,
    to: Option<String>,
    repository_url: Option<String>,
    version_name: Option<String>,
) -> Result<()> {
    // Load and apply configuration
    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;

    // Create a spinner to indicate progress
    let mut spinner = output::create_tui_spinner("Generating release notes...");

    // Check environment prerequisites
    if let Err(e) = config.check_environment() {
        output::print_error(&format!("Error: {e}"));
        output::print_info("\nPlease ensure the following:");
        output::print_info("1. Git is installed and accessible from the command line.");
        output::print_info(
            "2. You are running this command from within a Git repository or provide a repository URL with --repo.",
        );
        output::print_info("3. You have set up your configuration using 'git config'.");
        return Err(e);
    }

    // Use the repository URL from command line or common params
    let repo_url = repository_url.or(common.repository_url);

    // Create a GitRepo instance based on the URL or current directory
    let git_repo = if let Some(url) = repo_url {
        Arc::new(GitRepo::clone_remote_repository(&url).context("Failed to clone repository")?)
    } else {
        let repo_path = env::current_dir()?;
        Arc::new(GitRepo::new(&repo_path).context("Failed to create GitRepo")?)
    };

    // Set the default 'to' reference if not provided
    let to = to.unwrap_or_else(|| "HEAD".to_string());

    let detail_level = common.detail_level;

    // Generate the release notes
    let release_notes =
        ReleaseNotesGenerator::generate(git_repo, &from, &to, &config, detail_level, version_name)
            .await?;

    // Clear the spinner and display the result
    spinner.tick();

    // Output the release notes with decorative borders
    output::print_bordered_content(&release_notes);

    Ok(())
}
