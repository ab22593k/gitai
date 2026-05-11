#[allow(clippy::uninlined_format_args)]
pub mod models;
pub mod pr;
#[allow(clippy::uninlined_format_args)]
pub mod prompt;

use anyhow::{Context, Result};
use claw_core::common::CommonParams;
use claw_core::config::Config;
use claw_core::git::GitRepo;
use claw_core::llm::provider::ProviderKind;
use claw_core::output;
use std::env;
use std::sync::Arc;

pub async fn handle_pr_command(
    common: CommonParams,
    from: Option<String>,
    to: Option<String>,
    repository_url: Option<String>,
) -> Result<()> {
    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;

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

    let repo_url = repository_url.or(common.repository_url.clone());

    let git_repo = if let Some(url) = repo_url {
        Arc::new(GitRepo::clone_remote_repository(&url).context("Failed to clone repository")?)
    } else {
        let repo_path = env::current_dir()?;
        Arc::new(GitRepo::new(&repo_path).context("Failed to create GitRepo")?)
    };

    let effective_instructions = common
        .instructions
        .unwrap_or_else(|| config.instructions.clone());

    let provider_name = ProviderKind::Google.as_str();

    let pr_description = pr::generate_pr_based_on_parameters(
        git_repo,
        &effective_instructions,
        &config,
        provider_name,
        from,
        to,
    )
    .await?;

    println!("{}", models::format_pull_request(&pr_description));

    Ok(())
}
