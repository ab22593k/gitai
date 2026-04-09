use crate::commands::commit::completion::CompletionService;
use crate::commands::commit::service::CommitService;
use crate::common::{CommonParams, DetailLevel};
use crate::config::Config;
use crate::git::GitRepo;
use crate::llm::provider::ProviderKind;

use anyhow::{Context, Result};
use std::str::FromStr;
use std::sync::Arc;

pub fn create_commit_service(
    common: &CommonParams,
    repository_url: Option<String>,
    config: &Config,
) -> Result<Arc<CommitService>> {
    let repo_url = repository_url.or(common.repository_url.clone());

    let git_repo = GitRepo::new_from_url(repo_url).context("Failed to create GitRepo")?;

    let repo_path = git_repo.repo_path().clone();
    let provider_name = ProviderKind::Google.as_str();

    let detail_level = DetailLevel::from_str(&common.detail_level).unwrap_or(DetailLevel::Standard);

    let service = Arc::new(
        CommitService::new(
            config.clone(),
            &repo_path,
            provider_name,
            detail_level,
            git_repo,
        )
        .context("Failed to create CommitService")?,
    );

    service
        .check_environment()
        .context("Environment check failed")?;

    Ok(service)
}

pub fn create_completion_service(
    common: &CommonParams,
    repository_url: Option<String>,
    config: &Config,
) -> Result<Arc<CompletionService>> {
    let repo_url = repository_url.or(common.repository_url.clone());

    let git_repo = GitRepo::new_from_url(repo_url).context("Failed to create GitRepo")?;

    let repo_path = git_repo.repo_path().clone();
    let provider_name = ProviderKind::Google.as_str();

    let service = Arc::new(
        CompletionService::new(config.clone(), &repo_path, provider_name, git_repo)
            .context("Failed to create CompletionService")?,
    );

    service
        .check_environment()
        .context("Environment check failed")?;

    Ok(service)
}
