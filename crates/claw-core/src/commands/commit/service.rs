use super::git_service_core::GitServiceCore;
use super::strategy::{CommitMessageStrategy, CommitPromptStrategy, CompletionStrategy};
use super::types::GeneratedMessage;
use crate::common::DetailLevel;
use crate::config::Config;
use crate::git::{CommitResult, GitRepo};
use crate::llm::context::CommitContext;
use crate::llm::engine;

use anyhow::Result;
use log::debug;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::path::Path;
use tokio::sync::mpsc;

/// Service for handling Git commit operations with AI assistance
pub struct CommitService {
    core: GitServiceCore,
    detail_level: DetailLevel,
}

impl CommitService {
    /// Create a new `CommitService` instance
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
    pub fn get_git_info_for_commit(&self, commit_id: &str) -> Result<CommitContext> {
        debug!("Getting git info for commit: {commit_id}");
        let context = self
            .core
            .repo()
            .get_git_info_for_commit(self.core.config(), commit_id)?;
        Ok(context)
    }

    /// Generic method to generate AI content using a specific strategy
    async fn generate<T, S>(
        &self,
        strategy: S,
        instructions: &str,
        context: Option<CommitContext>,
    ) -> Result<T>
    where
        T: DeserializeOwned + JsonSchema,
        S: CommitPromptStrategy,
    {
        let mut config_clone = self.core.config_clone();
        config_clone.instructions = instructions.to_string();

        let context = if let Some(ctx) = context {
            ctx
        } else {
            self.core.get_git_info().await?
        };

        let system_prompt = strategy.create_system_prompt(&config_clone)?;
        let user_prompt = strategy.create_user_prompt(&context)?;

        engine::get_message::<T>(
            &config_clone,
            self.core.provider_name(),
            &system_prompt,
            &user_prompt,
        )
        .await
    }

    /// Generate a commit message using AI
    pub async fn generate_message(&self, instructions: &str) -> Result<GeneratedMessage> {
        let strategy = CommitMessageStrategy::new(self.detail_level);
        self.generate(strategy, instructions, None).await
    }

    /// Generate a commit message using AI with custom context
    pub async fn generate_message_with_context(
        &self,
        instructions: &str,
        context: CommitContext,
    ) -> Result<GeneratedMessage> {
        let strategy = CommitMessageStrategy::new(self.detail_level);
        self.generate(strategy, instructions, Some(context)).await
    }

    /// Generate a completion for a partially typed message
    pub async fn generate_completion(
        &self,
        prefix: &str,
        context_ratio: f32,
        instructions: &str,
    ) -> Result<GeneratedMessage> {
        let strategy = CompletionStrategy::new(prefix.to_string(), context_ratio);
        self.generate(strategy, instructions, None).await
    }

    /// Performs a commit with the given message.
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
