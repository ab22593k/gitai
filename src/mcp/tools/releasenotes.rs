//! MCP release notes tool implementation
//!
//! This module provides the MCP tool for generating release notes.

use crate::changes::ReleaseNotesGenerator;
use crate::config::Config as PilotConfig;
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

/// Release notes tool for generating comprehensive release notes
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct ReleaseNotesTool {
    /// Starting reference (commit hash, tag, or branch name)
    pub from: String,

    /// Ending reference (commit hash, tag, or branch name). Defaults to HEAD if not specified.
    #[serde(default)]
    pub to: String,

    /// Level of detail for the release notes
    #[serde(default)]
    pub detail_level: String,

    /// Custom instructions for the AI
    #[serde(default)]
    pub custom_instructions: String,

    /// Repository path (local) or URL (remote). Required.
    pub repository: String,

    /// Explicit version name to use (optional)
    #[serde(default)]
    pub version_name: String,
}

impl ReleaseNotesTool {
    /// Returns the tool definition for the release notes tool
    pub fn get_tool_definition() -> Tool {
        Tool {
            name: Cow::Borrowed("gitai_release_notes"),
            description: Some(Cow::Borrowed(
                "Generate comprehensive release notes between two Git references",
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
impl PilotTool for ReleaseNotesTool {
    /// Execute the release notes tool with the provided repository and configuration
    async fn execute(
        &self,
        git_repo: Arc<GitRepo>,
        config: PilotConfig,
    ) -> Result<CallToolResult, anyhow::Error> {
        debug!("Generating release notes with: {:?}", self);

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

        // Generate the release notes using the generator
        let content = ReleaseNotesGenerator::generate(
            git_repo.clone(),
            &self.from,
            &to,
            &config,
            detail_level,
            if self.version_name.is_empty() {
                None
            } else {
                Some(self.version_name.clone())
            },
        )
        .await?;

        // Create and return the result using shared utility
        Ok(create_text_result(content))
    }
}
