pub mod change_log;
#[allow(clippy::uninlined_format_args)]
pub mod prompt;

use crate::change_log::ChangelogGenerator;
use anyhow::{Context, Result, anyhow};
use claw_core::common::CommonParams;
use claw_core::config::Config;
use claw_core::git::GitRepo;
use claw_core::output;
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

    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;

    let mut spinner = output::create_tui_spinner("Generating changelog...");

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

    let repo_url = repository_url.or(common.repository_url);

    let git_repo = if let Some(url) = repo_url {
        Arc::new(GitRepo::clone_remote_repository(&url).context("Failed to clone repository")?)
    } else {
        let repo_path = env::current_dir()?;
        Arc::new(GitRepo::new(&repo_path).context("Failed to create GitRepo")?)
    };

    let git_repo_for_update = Arc::clone(&git_repo);

    let should_update_file = update_file || save;
    let changelog_file_path = if save {
        changelog_path.or_else(|| Some("CHANGELOG.md".to_string()))
    } else {
        changelog_path
    };

    let from_ref = if let Some(f) = from {
        Some(f)
    } else if save {
        output::print_info("Detecting latest tag...");
        match git_repo.get_latest_tag() {
            Ok(Some(tag)) => {
                output::print_success(&format!("Found latest tag: {tag}"));
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
                return Err(anyhow!("Failed to detect latest tag: {e}"));
            }
        }
    } else {
        None
    };

    let from_ref = from_ref
        .ok_or_else(|| anyhow!("Starting reference (--from) is required when not using --save"))?;

    let to = to.unwrap_or_else(|| "HEAD".to_string());

    let detail_level = common.detail_level;

    let changelog =
        ChangelogGenerator::generate(git_repo, &from_ref, &to, &config, detail_level).await?;

    spinner.tick();

    output::print_bordered_content(&changelog);

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
