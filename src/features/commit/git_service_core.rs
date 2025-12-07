use crate::config::Config;
use crate::core::context::CommitContext;
use crate::git::{CommitResult, GitRepo};

use anyhow::Result;
use log::debug;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Base functionality shared between `CommitService` and `CompletionService`
pub struct GitServiceCore {
    pub(crate) config: Config,
    pub(crate) repo: Arc<GitRepo>,
    pub(crate) provider_name: String,
    pub(crate) cached_context: Arc<RwLock<Option<CommitContext>>>,
}

impl GitServiceCore {
    /// Create a new `GitServiceCore` instance
    pub fn new(config: Config, provider_name: &str, git_repo: GitRepo) -> Self {
        Self {
            config,
            repo: Arc::new(git_repo),
            provider_name: provider_name.to_string(),
            cached_context: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if the repository is remote
    #[inline]
    pub fn is_remote_repository(&self) -> bool {
        self.repo.is_remote()
    }

    /// Check the environment for necessary prerequisites
    #[inline]
    pub fn check_environment(&self) -> Result<()> {
        self.config.check_environment()
    }

    /// Get Git information for the current repository (cached)
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

    /// Performs a commit with the given message.
    ///
    /// # Arguments
    ///
    /// * `message` - The commit message.
    /// * `amend` - Whether to amend the previous commit.
    /// * `commit_ref` - Optional commit reference for amend.
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
            "Performing commit with message: {message}, amend: {amend}, commit_ref: {commit_ref:?}"
        );

        // Execute pre-commit hook
        debug!("Executing pre-commit hook");
        if let Err(e) = self.repo.execute_hook("pre-commit") {
            debug!("Pre-commit hook failed: {e}");
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
                    debug!("Post-commit hook failed: {e}");
                    // We don't fail the commit if post-commit hook fails
                }
                debug!("Commit performed successfully");
                Ok(result)
            }
            Err(e) => {
                debug!("Commit failed: {e}");
                Err(e)
            }
        }
    }

    /// Get a reference to the config
    #[inline]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get a clone of the config
    #[inline]
    pub fn config_clone(&self) -> Config {
        self.config.clone()
    }

    /// Get the provider name
    #[inline]
    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    /// Get a reference to the repository
    #[inline]
    pub fn repo(&self) -> &GitRepo {
        &self.repo
    }
}
