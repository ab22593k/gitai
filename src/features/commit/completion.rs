#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::as_conversions)]

use super::git_service_core::GitServiceCore;
use super::prompt::{create_completion_system_prompt, create_completion_user_prompt};
use super::types::GeneratedMessage;
use crate::config::Config;
use crate::core::context::CommitContext;
use crate::core::llm;
use crate::git::{CommitResult, GitRepo};

use anyhow::Result;
use std::path::Path;
use tokio::sync::mpsc;

/// Service for handling Git commit message completion with AI assistance
pub struct CompletionService {
    core: GitServiceCore,
}

impl CompletionService {
    /// Create a new `CompletionService` instance
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration for the service
    /// * `repo_path` - The path to the Git repository (unused but kept for API compatibility)
    /// * `provider_name` - The name of the LLM provider to use
    /// * `git_repo` - An existing `GitRepo` instance
    ///
    /// # Returns
    ///
    /// A Result containing the new `CompletionService` instance or an error
    pub fn new(
        config: Config,
        _repo_path: &Path,
        provider_name: &str,
        git_repo: GitRepo,
    ) -> Result<Self> {
        Ok(Self {
            core: GitServiceCore::new(config, provider_name, git_repo),
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

    /// Generate a commit message completion using AI
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix text to complete
    /// * `context_ratio` - The ratio of the original message to use as context (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// A Result containing the generated completion or an error
    pub async fn complete_message(
        &self,
        prefix: &str,
        context_ratio: f32,
    ) -> anyhow::Result<GeneratedMessage> {
        let mut config_clone = self.core.config_clone();

        // Set instructions to include completion context
        let completion_instructions = format!(
            "Complete the commit message starting with the prefix: '{prefix}'. Use {}% of the original message as context.",
            (context_ratio * 100.0) as i32
        );
        config_clone.instructions = completion_instructions;

        let mut context = self.core.get_git_info().await?;

        // Enhance context with semantically similar history
        context.author_history = context.get_enhanced_history(10);

        // Create system prompt for completion
        let system_prompt = create_completion_system_prompt(&config_clone)?;

        // Generate user prompt directly
        let final_user_prompt = create_completion_user_prompt(&context, prefix, context_ratio);

        let generated_message = llm::get_message::<GeneratedMessage>(
            &config_clone,
            self.core.provider_name(),
            &system_prompt,
            &final_user_prompt,
        )
        .await?;

        Ok(generated_message)
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

    /// Create a channel for message completion
    pub fn create_completion_channel(
        &self,
    ) -> (
        mpsc::Sender<Result<GeneratedMessage>>,
        mpsc::Receiver<Result<GeneratedMessage>>,
    ) {
        mpsc::channel(1)
    }
}
