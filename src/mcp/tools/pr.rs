//! MCP PR description tool implementation
//!
//! This module provides the MCP tool for generating pull request descriptions.

use crate::commit::service::CommitService;
use crate::commit::types::format_pull_request;
use crate::config::Config as PilotConfig;
use crate::git::GitRepo;
use crate::debug;
use crate::mcp::tools::utils::{
    PilotTool, apply_custom_instructions, create_text_result, resolve_git_repo,
    validate_repository_parameter,
};

use rmcp::handler::server::tool::cached_schema_for_type;
use rmcp::model::{CallToolResult, Tool};
use rmcp::schemars;

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;

/// PR description tool for generating comprehensive pull request descriptions
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct PrTool {
    /// Starting reference (commit hash, tag, or branch name)
    pub from: String,

    /// Ending reference (commit hash, tag, or branch name). Defaults to HEAD if not specified.
    #[serde(default)]
    pub to: String,

    /// Preset instruction set to use for the PR description
    #[serde(default)]
    pub preset: String,

    /// Custom instructions for the AI
    #[serde(default)]
    pub custom_instructions: String,

    /// Repository path (local) or URL (remote). Required.
    pub repository: String,
}

impl PrTool {
    /// Returns the tool definition for the PR description tool
    pub fn get_tool_definition() -> Tool {
        Tool {
            name: Cow::Borrowed("gitpilot_pr"),
            description: Cow::Borrowed(
                "Generate comprehensive pull request descriptions for changesets spanning multiple commits. Analyzes commits and changes as an atomic unit.",
            ),
            input_schema: cached_schema_for_type::<Self>(),
        }
    }
}

#[async_trait::async_trait]
impl PilotTool for PrTool {
    /// Execute the PR description tool with the provided repository and configuration
    async fn execute(
        &self,
        git_repo: Arc<GitRepo>,
        config: PilotConfig,
    ) -> Result<CallToolResult, anyhow::Error> {
        debug!("Generating PR description with: {:?}", self);

        // Validate repository parameter
        validate_repository_parameter(&self.repository)?;
        let git_repo = resolve_git_repo(Some(self.repository.as_str()), git_repo)?;
        debug!("Using repository: {}", git_repo.repo_path().display());

        // Validate that we have both from and to parameters
        if self.from.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "The 'from' parameter is required for PR description generation"
            ));
        }

        // Default to HEAD if to is empty
        let to = if self.to.trim().is_empty() {
            "HEAD".to_string()
        } else {
            self.to.clone()
        };

        // Check if this is a remote repository (read-only mode)
        if git_repo.is_remote() {
            debug!("Operating on remote repository in read-only mode");
        }

        // Create a commit service for processing
        let repo_path = git_repo.repo_path().clone();
        let provider_name = &config.default_provider;

        let service = CommitService::new(
            config.clone(),
            &repo_path,
            provider_name,
            false, // emoji not needed for PR
            false, // verification not needed for PR
            GitRepo::new(&repo_path)?,
        )?;

        // Set up config with custom instructions if provided
        let mut config_clone = config.clone();
        apply_custom_instructions(&mut config_clone, &self.custom_instructions);

        // Process the preset
        let preset = if self.preset.trim().is_empty() {
            "default"
        } else {
            &self.preset
        };

        // Generate the PR description using the commit range
        let pr_description = service
            .generate_pr_for_commit_range(preset, &self.custom_instructions, &self.from, &to)
            .await?;

        // Format and return the PR description
        let formatted_pr = format_pull_request(&pr_description);
        Ok(create_text_result(formatted_pr))
    }
}
