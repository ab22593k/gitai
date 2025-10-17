//! Rebase module providing AI-assisted interactive rebase functionality

mod service;
mod types;

pub use service::RebaseService;
pub use types::{RebaseAction, RebaseAnalysis, RebaseCommit, RebaseResult};

use crate::common::CommonParams;
use crate::config::Config;
use crate::debug;
use crate::git::GitRepo;
use crate::ui;

use anyhow::Result;
use std::sync::Arc;

/// Handle the rebase command
pub async fn handle_rebase_command(
    common: CommonParams,
    upstream: String,
    branch: Option<String>,
    auto_apply: bool,
    commit_types: Option<String>,
    repository_url: Option<String>,
) -> Result<()> {
    debug!(
        "Handling rebase command: upstream={}, branch={:?}, auto_apply={}, commit_types={:?}",
        upstream, branch, auto_apply, commit_types
    );

    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;

    // Create the service
    let service = create_rebase_service(&common, repository_url, &config)?;

    // Analyze commits for rebase
    let analysis = service.analyze_commits_for_rebase(&upstream, branch.as_deref()).await?;

    if analysis.commits.is_empty() {
        ui::print_info("No commits to rebase. Branch is already up to date.");
        return Ok(());
    }

    ui::print_info(&format!("Found {} commits to rebase", analysis.commits.len()));

    if auto_apply {
        // Auto-apply AI suggestions
        ui::print_info("Auto-applying AI suggestions...");
        let result = service.perform_rebase_auto(analysis).await?;
        ui::print_success(&format!("Rebase completed successfully with {} operations", result.operations_performed));
    } else {
        // Interactive mode - for now, just show what would be done
        ui::print_warning("Interactive rebase mode is not yet implemented.");
        ui::print_info("Showing AI suggestions:");

        for (i, commit) in analysis.commits.iter().enumerate() {
            ui::print_info(&format!("{}. {} - Suggested: {:?}",
                i + 1,
                &commit.message.lines().next().unwrap_or(""),
                commit.suggested_action
            ));
        }

        ui::print_info("Use --auto-apply to automatically apply these suggestions.");
    }

    Ok(())
}

/// Create a RebaseService instance
fn create_rebase_service(
    common: &CommonParams,
    repository_url: Option<String>,
    config: &Config,
) -> Result<Arc<RebaseService>> {
    // Combine repository URL from CLI and CommonParams
    let repo_url = repository_url.or(common.repository_url.clone());

    // Create the git repository
    let git_repo = GitRepo::new_from_url(repo_url)?;

    let service = Arc::new(RebaseService::new(
        config.clone(),
        git_repo,
    )?);

    // Check environment prerequisites
    service.check_environment()?;

    Ok(service)
}