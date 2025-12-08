use super::git_service_core::GitServiceCore;
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
use tokio::sync::mpsc;

/// Service for handling Git commit operations with AI assistance
pub struct CommitService {
    core: GitServiceCore,
    detail_level: DetailLevel,
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
            core: GitServiceCore::new(config, provider_name, git_repo),
            detail_level,
        })
    }

    /// Check if the repository is remote
    #[inline]
    pub fn is_remote_repository(&self) -> bool {
        self.core.is_remote_repository()
    }

    /// Check the environment for necessary prerequisites
    #[inline]
    pub fn check_environment(&self) -> Result<()> {
        self.core.check_environment()
    }

    /// Get Git information for the current repository
    #[inline]
    pub async fn get_git_info(&self) -> Result<CommitContext> {
        self.core.get_git_info().await
    }

    /// Get Git information including unstaged changes
    #[inline]
    pub async fn get_git_info_with_unstaged(
        &self,
        include_unstaged: bool,
    ) -> Result<CommitContext> {
        self.core.get_git_info_with_unstaged(include_unstaged).await
    }

    /// Get Git information for a specific commit
    #[allow(clippy::unused_async)]
    pub async fn get_git_info_for_commit(&self, commit_id: &str) -> Result<CommitContext> {
        debug!("Getting git info for commit: {commit_id}");
        let context = self
            .core
            .repo()
            .get_git_info_for_commit(self.core.config(), commit_id)?;
        Ok(context)
    }

    /// Generate a commit message using AI
    ///
    /// # Arguments
    ///
    /// * `instructions` - Custom instructions for the AI
    ///
    /// # Returns
    ///
    /// A Result containing the generated commit message or an error
    pub async fn generate_message(&self, instructions: &str) -> anyhow::Result<GeneratedMessage> {
        let mut config_clone = self.core.config_clone();
        config_clone.instructions = instructions.to_string();

        let context = self.core.get_git_info().await?;

        // Create system prompt
        let system_prompt = create_system_prompt(&config_clone)?;

        // Generate user prompt directly
        let final_user_prompt = create_user_prompt(&context, self.detail_level);

        let generated_message = llm::get_message::<GeneratedMessage>(
            &config_clone,
            self.core.provider_name(),
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
        let mut config_clone = self.core.config_clone();
        config_clone.instructions = instructions.to_string();

        // Create system prompt
        let system_prompt = create_system_prompt(&config_clone)?;

        // Generate user prompt directly
        let final_user_prompt = create_user_prompt(&context, self.detail_level);

        let generated_message = llm::get_message::<GeneratedMessage>(
            &config_clone,
            self.core.provider_name(),
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
        let mut config_clone = self.core.config_clone();
        config_clone.instructions = instructions.to_string();

        // Get context for the commit range
        let context =
            self.core
                .repo()
                .get_git_info_for_commit_range(self.core.config(), from, to)?;

        // Get commit messages for the PR
        let commit_messages = self.core.repo().get_commits_for_pr(from, to)?;

        // Create system prompt
        let system_prompt = super::prompt::create_pr_system_prompt(&config_clone)?;

        // Generate user prompt directly
        let final_user_prompt = super::prompt::create_pr_user_prompt(&context, &commit_messages);

        let generated_pr = llm::get_message::<super::types::GeneratedPullRequest>(
            &config_clone,
            self.core.provider_name(),
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
        let mut config_clone = self.core.config_clone();
        config_clone.instructions = instructions.to_string();

        // Get context for the branch comparison
        let context = self.core.repo().get_git_info_for_branch_diff(
            self.core.config(),
            base_branch,
            target_branch,
        )?;

        // Get commit messages for the PR (commits in target_branch not in base_branch)
        let commit_messages = self
            .core
            .repo()
            .get_commits_for_pr(base_branch, target_branch)?;

        // Create system prompt
        let system_prompt = super::prompt::create_pr_system_prompt(&config_clone)?;

        // Generate user prompt directly
        let final_user_prompt = super::prompt::create_pr_user_prompt(&context, &commit_messages);

        let generated_pr = llm::get_message::<super::types::GeneratedPullRequest>(
            &config_clone,
            self.core.provider_name(),
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
    /// * `amend` - Whether to amend the previous commit.
    /// * `commit_ref` - Optional commit reference for amending.
    ///
    /// # Returns
    ///
    /// A Result containing the `CommitResult` or an error.
    #[inline]
    pub fn perform_commit(
        &self,
        message: &str,
        amend: bool,
        commit_ref: Option<&str>,
    ) -> Result<CommitResult> {
        self.core.perform_commit(message, amend, commit_ref)
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
