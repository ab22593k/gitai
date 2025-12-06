use super::prompt::{create_system_prompt, create_user_prompt};
use super::types::GeneratedMessage;
use crate::common::DetailLevel;
use crate::config::Config;
use crate::core::context::CommitContext;
use crate::core::llm;
use crate::git::{CommitResult, GitRepo};

use anyhow::Result;
use log::debug;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

/// Service for handling Git commit operations with AI assistance
pub struct CommitService {
    config: Config,
    repo: Arc<GitRepo>,
    provider_name: String,
    detail_level: DetailLevel,
    cached_context: Arc<RwLock<Option<CommitContext>>>,
}

impl CommitService {
    /// Create a new `CommitService` instance
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration for the service
    /// * `repo_path` - The path to the Git repository (unused but kept for API compatibility)
    /// * `provider_name` - The name of the LLM provider to use
    /// * `detail_level` - The level of detail for generated messages
    /// * `git_repo` - An existing `GitRepo` instance
    ///
    /// # Returns
    ///
    /// A Result containing the new `CommitService` instance or an error
    pub fn new(
        config: Config,
        _repo_path: &Path,
        provider_name: &str,
        detail_level: DetailLevel,
        git_repo: GitRepo,
    ) -> Result<Self> {
        Ok(Self {
            config,
            repo: Arc::new(git_repo),
            provider_name: provider_name.to_string(),
            detail_level,
            cached_context: Arc::new(RwLock::new(None)),
        })
    }

    /// Check if the repository is remote
    pub fn is_remote_repository(&self) -> bool {
        self.repo.is_remote()
    }

    /// Check the environment for necessary prerequisites
    pub fn check_environment(&self) -> Result<()> {
        self.config.check_environment()
    }

    /// Get Git information for the current repository
    pub async fn get_git_info(&self) -> Result<CommitContext> {
        {
            let cached_context = self.cached_context.read().await;
            if let Some(context) = &*cached_context {
                return Ok(context.clone());
            }
        }

        let context = self.repo.get_git_info(&self.config).await?;

        {
            let mut cached_context = self.cached_context.write().await;
            *cached_context = Some(context.clone());
        }
        Ok(context)
    }

    /// Get Git information including unstaged changes
    pub async fn get_git_info_with_unstaged(
        &self,
        include_unstaged: bool,
    ) -> Result<CommitContext> {
        if !include_unstaged {
            return self.get_git_info().await;
        }

        {
            // Only use cached context if we're not including unstaged changes
            // because unstaged changes might have changed since we last checked
            let cached_context = self.cached_context.read().await;
            if let Some(context) = &*cached_context
                && !include_unstaged
            {
                return Ok(context.clone());
            }
        }

        let context = self
            .repo
            .get_git_info_with_unstaged(&self.config, include_unstaged)
            .await?;

        // Don't cache the context with unstaged changes since they can be constantly changing
        if !include_unstaged {
            let mut cached_context = self.cached_context.write().await;
            *cached_context = Some(context.clone());
        }

        Ok(context)
    }

    /// Get Git information for a specific commit
    #[allow(clippy::unused_async)]
    pub async fn get_git_info_for_commit(&self, commit_id: &str) -> Result<CommitContext> {
        debug!("Getting git info for commit: {}", commit_id);

        let context = self.repo.get_git_info_for_commit(&self.config, commit_id)?;

        // We don't cache commit-specific contexts
        Ok(context)
    }

    /// Generate a commit message using AI
    ///
    /// # Arguments
    ///
    /// * `preset` - The instruction preset to use
    /// * `instructions` - Custom instructions for the AI
    ///
    /// # Returns
    ///
    /// A Result containing the generated commit message or an error
    pub async fn generate_message(&self, instructions: &str) -> anyhow::Result<GeneratedMessage> {
        let mut config_clone = self.config.clone();

        config_clone.instructions = instructions.to_string();

        let context = self.get_git_info().await?;

        // Create system prompt
        let system_prompt = create_system_prompt(&config_clone)?;

        // Use the shared optimization logic
        let (_, final_user_prompt) = super::prompt_optimizer::optimize_prompt(
            &config_clone,
            &self.provider_name,
            &system_prompt,
            context,
            |ctx| create_user_prompt(ctx, self.detail_level),
        )
        .await;

        let generated_message = llm::get_message::<GeneratedMessage>(
            &config_clone,
            &self.provider_name,
            &system_prompt,
            &final_user_prompt,
        )
        .await?;

        Ok(generated_message)
    }

    /// Generate a commit message using AI with custom context
    ///
    /// # Arguments
    ///
    /// * `instructions` - Custom instructions for the AI
    /// * `context` - The context to use for generation
    ///
    /// # Returns
    ///
    /// A Result containing the generated message or an error
    pub async fn generate_message_with_context(
        &self,
        instructions: &str,
        context: CommitContext,
    ) -> anyhow::Result<GeneratedMessage> {
        let mut config_clone = self.config.clone();

        config_clone.instructions = instructions.to_string();

        // Create system prompt
        let system_prompt = create_system_prompt(&config_clone)?;

        // Use the shared optimization logic with provided context
        let (_, final_user_prompt) = super::prompt_optimizer::optimize_prompt(
            &config_clone,
            &self.provider_name,
            &system_prompt,
            context,
            |ctx| create_user_prompt(ctx, self.detail_level),
        )
        .await;

        let generated_message = llm::get_message::<GeneratedMessage>(
            &config_clone,
            &self.provider_name,
            &system_prompt,
            &final_user_prompt,
        )
        .await?;

        Ok(generated_message)
    }

    /// Generate a PR description for a commit range
    ///
    /// # Arguments
    ///
    /// * `preset` - The instruction preset to use
    /// * `instructions` - Custom instructions for the AI
    /// * `from` - The starting Git reference (exclusive)
    /// * `to` - The ending Git reference (inclusive)
    ///
    /// # Returns
    ///
    /// A Result containing the generated PR description or an error
    pub async fn generate_pr_for_commit_range(
        &self,
        instructions: &str,
        from: &str,
        to: &str,
    ) -> anyhow::Result<super::types::GeneratedPullRequest> {
        let mut config_clone = self.config.clone();

        config_clone.instructions = instructions.to_string();

        // Get context for the commit range
        let context = self
            .repo
            .get_git_info_for_commit_range(&self.config, from, to)?;

        // Get commit messages for the PR
        let commit_messages = self.repo.get_commits_for_pr(from, to)?;

        // Create system prompt
        let system_prompt = super::prompt::create_pr_system_prompt(&config_clone)?;

        // Use the shared optimization logic
        let (_, final_user_prompt) = super::prompt_optimizer::optimize_prompt(
            &config_clone,
            &self.provider_name,
            &system_prompt,
            context,
            |ctx| super::prompt::create_pr_user_prompt(ctx, &commit_messages),
        )
        .await;

        let generated_pr = llm::get_message::<super::types::GeneratedPullRequest>(
            &config_clone,
            &self.provider_name,
            &system_prompt,
            &final_user_prompt,
        )
        .await?;

        Ok(generated_pr)
    }

    /// Generate a PR description for branch comparison
    ///
    /// # Arguments
    ///
    /// * `preset` - The instruction preset to use
    /// * `instructions` - Custom instructions for the AI
    /// * `base_branch` - The base branch (e.g., "main")
    /// * `target_branch` - The target branch (e.g., "feature-branch")
    ///
    /// # Returns
    ///
    /// A Result containing the generated PR description or an error
    pub async fn generate_pr_for_branch_diff(
        &self,
        instructions: &str,
        base_branch: &str,
        target_branch: &str,
    ) -> anyhow::Result<super::types::GeneratedPullRequest> {
        let mut config_clone = self.config.clone();

        config_clone.instructions = instructions.to_string();

        // Get context for the branch comparison
        let context =
            self.repo
                .get_git_info_for_branch_diff(&self.config, base_branch, target_branch)?;

        // Get commit messages for the PR (commits in target_branch not in base_branch)
        let commit_messages = self.repo.get_commits_for_pr(base_branch, target_branch)?;

        // Create system prompt
        let system_prompt = super::prompt::create_pr_system_prompt(&config_clone)?;

        // Use the shared optimization logic
        let (_, final_user_prompt) = super::prompt_optimizer::optimize_prompt(
            &config_clone,
            &self.provider_name,
            &system_prompt,
            context,
            |ctx| super::prompt::create_pr_user_prompt(ctx, &commit_messages),
        )
        .await;

        let generated_pr = llm::get_message::<super::types::GeneratedPullRequest>(
            &config_clone,
            &self.provider_name,
            &system_prompt,
            &final_user_prompt,
        )
        .await?;

        Ok(generated_pr)
    }

    /// Performs a commit with the given message.
    ///
    /// # Arguments
    ///
    /// * `message` - The commit message.
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitResult` or an error.
    pub fn perform_commit(
        &self,
        message: &str,
        amend: bool,
        commit_ref: Option<&str>,
    ) -> Result<CommitResult> {
        // Check if this is a remote repository
        if self.is_remote_repository() {
            return Err(anyhow::anyhow!("Cannot commit to a remote repository"));
        }

        debug!(
            "Performing commit with message: {}, amend: {}, commit_ref: {:?}",
            message, amend, commit_ref
        );

        // Execute pre-commit hook
        debug!("Executing pre-commit hook");
        if let Err(e) = self.repo.execute_hook("pre-commit") {
            debug!("Pre-commit hook failed: {}", e);
            return Err(e);
        }
        debug!("Pre-commit hook executed successfully");

        // Perform the commit
        let commit_result = if amend {
            self.repo
                .amend_commit(message, commit_ref.unwrap_or("HEAD"))
        } else {
            self.repo.commit(message)
        };

        match commit_result {
            Ok(result) => {
                // Execute post-commit hook
                debug!("Executing post-commit hook");
                if let Err(e) = self.repo.execute_hook("post-commit") {
                    debug!("Post-commit hook failed: {}", e);
                    // We don't fail the commit if post-commit hook fails
                }
                debug!("Commit performed successfully");
                Ok(result)
            }
            Err(e) => {
                debug!("Commit failed: {}", e);
                Err(e)
            }
        }
    }

    /// Create a channel for message generation
    pub fn create_message_channel(
        &self,
    ) -> (
        mpsc::Sender<Result<GeneratedMessage>>,
        mpsc::Receiver<Result<GeneratedMessage>>,
    ) {
        mpsc::channel(1)
    }
}
