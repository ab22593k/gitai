use log::debug;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the structured response for a changelog
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct ChangelogResponse {
    /// The version number of the release
    pub version: Option<String>,
    /// The date of the release
    pub release_date: Option<String>,
    /// Categorized changes, grouped by type
    pub sections: HashMap<ChangelogType, Vec<ChangeEntry>>,
    /// List of breaking changes in this release
    pub breaking_changes: Vec<BreakingChange>,
    /// Metrics summarizing the changes in this release
    pub metrics: ChangeMetrics,
}

/// Enumeration of possible change types for changelog entries
#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug, PartialEq, Eq, Hash)]
pub enum ChangelogType {
    Added,
    Changed,
    Deprecated,
    Removed,
    Fixed,
    Security,
}

/// Represents a single change entry in the changelog
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct ChangeEntry {
    /// Description of the change
    pub description: String,
    /// List of commit hashes associated with this change
    pub commit_hashes: Vec<String>,
    /// List of issue numbers associated with this change
    pub associated_issues: Vec<String>,
    /// Pull request number associated with this change, if any
    pub pull_request: Option<String>,
}

/// Represents a breaking change in the release
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct BreakingChange {
    /// Description of the breaking change
    pub description: String,
    /// Commit hash associated with this breaking change
    pub commit_hash: String,
}

/// Metrics summarizing the changes in a release
#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct ChangeMetrics {
    /// Total number of commits in this release
    pub total_commits: usize,
    /// Number of files changed in this release
    pub files_changed: usize,
    /// Number of lines inserted in this release
    pub insertions: usize,
    /// Number of lines deleted in this release
    pub deletions: usize,
    /// Total lines changed in this release
    pub total_lines_changed: usize,
}

impl From<String> for ChangelogResponse {
    /// Converts a JSON string to a `ChangelogResponse`
    fn from(value: String) -> Self {
        serde_json::from_str(&value).unwrap_or_else(|e| {
            debug!("Failed to parse ChangelogResponse: {e}");
            Self {
                version: Some("Error".to_string()),
                release_date: Some("Error".to_string()),
                sections: HashMap::new(),
                breaking_changes: Vec::new(),
                metrics: ChangeMetrics {
                    total_commits: 0,
                    files_changed: 0,
                    insertions: 0,
                    deletions: 0,
                    total_lines_changed: 0,
                },
            }
        })
    }
}
