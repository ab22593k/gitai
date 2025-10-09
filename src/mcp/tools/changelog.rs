//! MCP changelog tool implementation
//!
//! This module provides the MCP tool for generating changelogs.

use crate::changes::ChangelogGenerator;
use crate::config::Config as GitAIConfig;
use crate::debug;
use crate::git::GitRepo;
use crate::mcp::tools::utils::{
    PilotTool, apply_custom_instructions, create_text_result, parse_detail_level, resolve_git_repo,
    validate_repository_parameter,
};

use rmcp::handler::server::tool::cached_schema_for_type;
use rmcp::model::{CallToolResult, Tool};
use rmcp::schemars;

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;

/// Changelog tool for generating comprehensive changelog documents
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct ChangelogTool {
    /// Starting reference (commit hash, tag, or branch name)
    pub from: String,

    /// Ending reference (commit hash, tag, or branch name). Defaults to HEAD if not specified.
    #[serde(default)]
    pub to: String,

    /// Level of detail for the changelog
    #[serde(default)]
    pub detail_level: String,

    /// Custom instructions for the AI
    #[serde(default)]
    pub custom_instructions: String,

    /// Repository path (local) or URL (remote). Required.
    pub repository: String,

    /// Explicit version name to use in the changelog (optional)
    #[serde(default)]
    pub version_name: String,
}

impl ChangelogTool {
    /// Returns the tool definition for the changelog tool
    pub fn get_tool_definition() -> Tool {
        Tool {
            name: Cow::Borrowed("gitai_changelog"),
            description: Some(Cow::Borrowed(
                "Generate a detailed changelog between two Git references",
            )),
            input_schema: cached_schema_for_type::<Self>(),
            annotations: None,
            icons: None,
            output_schema: None,
            title: None,
        }
    }
}

#[async_trait::async_trait]
impl PilotTool for ChangelogTool {
    /// Execute the changelog tool with the provided repository and configuration
    async fn execute(
        &self,
        git_repo: Arc<GitRepo>,
        config: GitAIConfig,
    ) -> Result<CallToolResult, anyhow::Error> {
        debug!("Generating changelog with: {:?}", self);

        // Validate repository parameter
        validate_repository_parameter(&self.repository)?;
        let git_repo = resolve_git_repo(Some(self.repository.as_str()), git_repo)?;
        debug!("Using repository: {}", git_repo.repo_path().display());

        // Parse detail level using shared utility
        let detail_level = parse_detail_level(&self.detail_level);

        // Set up config with custom instructions if provided
        let mut config = config.clone();
        apply_custom_instructions(&mut config, &self.custom_instructions);

        // Default to HEAD if to is empty
        let to = if self.to.trim().is_empty() {
            "HEAD".to_string()
        } else {
            self.to.clone()
        };

        // Generate the changelog using the generator
        let content =
            ChangelogGenerator::generate(git_repo.clone(), &self.from, &to, &config, detail_level)
                .await?;

        // If version_name is provided, update the changelog content to use that version
        let version_opt = if self.version_name.is_empty() {
            None
        } else {
            Some(self.version_name.clone())
        };

        // If version_name is provided, use it when updating the changelog
        if let Some(version) = &version_opt {
            debug!("Using custom version name: {}", version);
        }

        // Create and return the result using shared utility
        Ok(create_text_result(content))
    }
}
