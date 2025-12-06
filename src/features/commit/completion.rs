#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::as_conversions)]

use super::prompt::{create_completion_system_prompt, create_completion_user_prompt};
use super::types::GeneratedMessage;
use crate::config::Config;
use crate::core::context::CommitContext;
use crate::core::llm;
use crate::git::{CommitResult, GitRepo};

use anyhow::Result;
use log::debug;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

/// Service for handling Git commit message completion with AI assistance
pub struct CompletionService {
    config: Config,
    repo: Arc<GitRepo>,
    provider_name: String,
    cached_context: Arc<RwLock<Option<CommitContext>>>,
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
            config,
            repo: Arc::new(git_repo),
            provider_name: provider_name.to_string(),
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
        let mut config_clone = self.config.clone();

        // Set instructions to include completion context
        let completion_instructions = format!(
            "Complete the commit message starting with the prefix: '{}'. Use {}% of the original message as context.",
            prefix,
            (context_ratio * 100.0) as i32
        );
        config_clone.instructions = completion_instructions;

        let mut context = self.get_git_info().await?;

        // Enhance context with semantically similar history
        context.author_history = context.get_enhanced_history(10);

        // Create system prompt for completion
        let system_prompt = create_completion_system_prompt(&config_clone)?;

        // Use the shared optimization logic
        let (_, final_user_prompt) = super::prompt_optimizer::optimize_prompt(
            &config_clone,
            &self.provider_name,
            &system_prompt,
            context,
            |ctx| create_completion_user_prompt(ctx, prefix, context_ratio),
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
