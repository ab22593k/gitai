//! Common utilities for MCP tools
//!
//! This module provides shared functionality used across different MCP tool implementations.

use crate::common::DetailLevel;
use crate::config::Config as PilotConfig;
use crate::git::GitRepo;
use rmcp::model::{Annotated, CallToolResult, Content, RawContent, RawTextContent};
use std::sync::Arc;

/// Common trait for all `GitAI` MCP tools
///
/// This trait defines the common interface that all `GitAI` tools must implement.
#[async_trait::async_trait]
pub trait PilotTool {
    /// Execute the tool with the provided repository and configuration
    async fn execute(
        &self,
        git_repo: Arc<GitRepo>,
        config: PilotConfig,
    ) -> Result<CallToolResult, anyhow::Error>;
}

/// Creates a text result response for tool calls
///
/// This is a common utility used by all tools to return a text response.
pub fn create_text_result(text: String) -> CallToolResult {
    CallToolResult {
        content: vec![Content::from(Annotated {
            raw: RawContent::Text(RawTextContent {
                text,
                meta: Option::default(),
            }),
            annotations: None,
        })],
        is_error: None,
        meta: Option::default(),
        structured_content: None,
    }
}

/// Parses a detail level string into the corresponding enum value
///
/// This provides consistent handling of detail level across all tools.
pub fn parse_detail_level(detail_level: &str) -> DetailLevel {
    if detail_level.trim().is_empty() {
        return DetailLevel::Standard;
    }

    match detail_level.trim().to_lowercase().as_str() {
        "minimal" => DetailLevel::Minimal,
        "detailed" => DetailLevel::Detailed,
        _ => DetailLevel::Standard,
    }
}

/// Apply custom instructions to config if provided
pub fn apply_custom_instructions(config: &mut crate::config::Config, custom_instructions: &str) {
    if !custom_instructions.trim().is_empty() {
        config.set_temp_instructions(Some(custom_instructions.to_string()));
    }
}

/// Validates the repository parameter: must be non-empty, and if local, must exist and be a git repo
pub fn validate_repository_parameter(repo: &str) -> Result<(), anyhow::Error> {
    if repo.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "The `repository` parameter is required and must be a valid local path or remote URL."
        ));
    }
    if !(repo.starts_with("http://") || repo.starts_with("https://") || repo.starts_with("git@")) {
        let path = std::path::Path::new(repo);
        if !path.exists() {
            return Err(anyhow::anyhow!(format!(
                "The specified repository path does not exist: {repo}"
            )));
        }
        if !path.join(".git").exists() {
            return Err(anyhow::anyhow!(format!(
                "The specified path is not a git repository: {repo}"
            )));
        }
    }
    Ok(())
}

/// Resolves a Git repository from a `repo_path` parameter
///
/// If `repo_path` is provided, creates a new `GitRepo` for that path/URL.
/// Assumes the parameter has already been validated.
pub fn resolve_git_repo(
    repo_path: Option<&str>,
    _default_git_repo: Arc<GitRepo>,
) -> Result<Arc<GitRepo>, anyhow::Error> {
    match repo_path {
        Some(path) if !path.trim().is_empty() => {
            if path.starts_with("http://")
                || path.starts_with("https://")
                || path.starts_with("git@")
            {
                // Handle remote repository URL
                Ok(Arc::new(GitRepo::new_from_url(Some(path.to_string()))?))
            } else {
                // Handle local repository path
                let path = std::path::Path::new(path);
                Ok(Arc::new(GitRepo::new(path)?))
            }
        }
        _ => Err(anyhow::anyhow!(
            "The `repository` parameter is required and must be a valid local path or remote URL."
        )),
    }
}
