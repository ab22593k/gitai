use super::prompt_helpers;
use super::types::GeneratedMessage;
use crate::common::{DetailLevel, get_combined_instructions};
use crate::config::Config;
use crate::llm::context::CommitContext;
use anyhow::Result;
use prompts::commit as commit_prompts;

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
        let schema = schemars::schema_for!(GeneratedMessage);
        let schema_str = serde_json::to_string_pretty(&schema)?;
        let instructions = get_combined_instructions(config);
        Ok(commit_prompts::create_system_prompt(
            &instructions,
            &schema_str,
        ))
    }

    fn create_user_prompt(&self, context: &CommitContext) -> Result<String> {
        let detail_instruction = match self.detail_level {
            DetailLevel::Minimal => {
                "EXIGENCY: Keep it technical and concise. A subsystem subject and a single paragraph of technical reasoning."
            }
            DetailLevel::Standard => {
                "EXIGENCY: Provide a multi-paragraph technical justification explaining the problem and solution."
            }
            DetailLevel::Detailed => {
                "EXIGENCY: Exhaustive technical documentation. Explain the state before/after, the logic flow, and architectural implications."
            }
        };

        Ok(commit_prompts::create_user_prompt(
            &context.branch,
            &prompt_helpers::format_staged_files(&context.staged_files),
            &prompt_helpers::format_detailed_changes(&context.staged_files),
            &prompt_helpers::format_recent_commits(&context.recent_commits),
            &prompt_helpers::format_enhanced_author_history(&context.author_history, context),
            detail_instruction,
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
        let schema = schemars::schema_for!(GeneratedMessage);
        let schema_str = serde_json::to_string_pretty(&schema)?;
        let instructions = get_combined_instructions(config);
        Ok(commit_prompts::create_completion_system_prompt(
            &instructions,
            &schema_str,
        ))
    }

    fn create_user_prompt(&self, context: &CommitContext) -> Result<String> {
        Ok(commit_prompts::create_completion_user_prompt(
            &self.prefix,
            self.context_ratio,
            &context.branch,
            &prompt_helpers::format_staged_files(&context.staged_files),
            &prompt_helpers::format_detailed_changes(&context.staged_files),
            &prompt_helpers::format_recent_commits(&context.recent_commits),
            &prompt_helpers::format_enhanced_author_history(&context.author_history, context),
        ))
    }
}
