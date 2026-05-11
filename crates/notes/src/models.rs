use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use claw_core::commands::changelog::models::{BreakingChange, ChangeMetrics};

/// Represents the structured response for release notes
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct ReleaseNotesResponse {
    /// The version number of the release
    pub version: Option<String>,
    /// The date of the release
    pub release_date: Option<String>,
    /// A brief summary of the release
    pub summary: String,
    /// List of highlighted changes or features in this release
    pub highlights: Vec<Highlight>,
    /// Detailed sections of changes
    pub sections: Vec<Section>,
    /// List of breaking changes in this release
    pub breaking_changes: Vec<BreakingChange>,
    /// Notes for upgrading to this version
    pub upgrade_notes: Vec<String>,
    /// Metrics summarizing the changes in this release
    pub metrics: ChangeMetrics,
}

/// Represents a highlight in the release notes
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct Highlight {
    /// Title of the highlight
    pub title: String,
    /// Detailed description of the highlight
    pub description: String,
}

/// Represents a section in the release notes
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct Section {
    /// Title of the section
    pub title: String,
    /// List of items in this section
    pub items: Vec<SectionItem>,
}

/// Represents an item in a section of the release notes
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct SectionItem {
    /// Description of the change
    pub description: String,
    /// List of issue numbers associated with this change
    pub associated_issues: Vec<String>,
    /// Pull request number associated with this change, if any
    pub pull_request: Option<String>,
}

impl From<String> for ReleaseNotesResponse {
    fn from(value: String) -> Self {
        serde_json::from_str(&value).unwrap_or_else(|e| {
            log::debug!("Failed to parse ReleaseNotesResponse: {e}");
            Self {
                version: Some("Error".to_string()),
                release_date: Some("Error".to_string()),
                summary: format!("Error parsing response: {e}"),
                highlights: Vec::new(),
                sections: Vec::new(),
                breaking_changes: Vec::new(),
                upgrade_notes: Vec::new(),
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
