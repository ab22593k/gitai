pub mod models;
pub mod notes;
pub mod prompt;

use crate::notes::ReleaseNotesGenerator;
use anyhow::{Context, Result};
use claw_core::common::CommonParams;
use claw_core::config::Config;
use claw_core::git::GitRepo;
use claw_core::output;
use std::env;
use std::sync::Arc;

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
