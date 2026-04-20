use crate::common::DetailLevel;
use crate::config::Config;
use crate::llm::context::CommitContext;
use anyhow::Result;

/// Trait for defining how to generate prompts for commit-related operations
pub trait CommitPromptStrategy: Send + Sync {
    /// Create the system prompt for the operation
    fn create_system_prompt(&self, config: &Config) -> Result<String>;

    /// Create the user prompt for the operation
    fn create_user_prompt(&self, context: &CommitContext) -> Result<String>;
}

/// Strategy for generating standard commit messages
pub struct CommitMessageStrategy {
    pub detail_level: DetailLevel,
}

impl CommitMessageStrategy {
    pub fn new(detail_level: DetailLevel) -> Self {
        Self { detail_level }
    }
}

impl CommitPromptStrategy for CommitMessageStrategy {
    fn create_system_prompt(&self, config: &Config) -> Result<String> {
        super::prompt::create_system_prompt(config)
    }

    fn create_user_prompt(&self, context: &CommitContext) -> Result<String> {
        Ok(super::prompt::create_user_prompt(
            context,
            self.detail_level,
        ))
    }
}

/// Strategy for generating pull request descriptions
pub struct PullRequestStrategy {
    pub commit_messages: Vec<String>,
}

impl PullRequestStrategy {
    pub fn new(commit_messages: Vec<String>) -> Self {
        Self { commit_messages }
    }
}

impl CommitPromptStrategy for PullRequestStrategy {
    fn create_system_prompt(&self, config: &Config) -> Result<String> {
        super::prompt::create_pr_system_prompt(config)
    }

    fn create_user_prompt(&self, context: &CommitContext) -> Result<String> {
        Ok(super::prompt::create_pr_user_prompt(
            context,
            &self.commit_messages,
        ))
    }
}

/// Strategy for completing partially typed commit messages
pub struct CompletionStrategy {
    pub prefix: String,
    pub context_ratio: f32,
}

impl CompletionStrategy {
    pub fn new(prefix: String, context_ratio: f32) -> Self {
        Self {
            prefix,
            context_ratio,
        }
    }
}

impl CommitPromptStrategy for CompletionStrategy {
    fn create_system_prompt(&self, config: &Config) -> Result<String> {
        super::prompt::create_completion_system_prompt(config)
    }

    fn create_user_prompt(&self, context: &CommitContext) -> Result<String> {
        Ok(super::prompt::create_completion_user_prompt(
            context,
            &self.prefix,
            self.context_ratio,
        ))
    }
}
