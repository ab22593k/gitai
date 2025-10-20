use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use textwrap;

/// Model for commit message generation results
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct GeneratedMessage {
    /// Commit message title/subject line
    pub title: String,
    /// Detailed commit message body
    pub message: String,
}

/// Model for pull request description generation results
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct GeneratedPullRequest {
    /// Pull request title
    pub title: String,
    /// Brief summary of the changes
    pub summary: String,
    /// Detailed description of what was changed and why
    pub description: String,
    /// List of commit messages included in this PR
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commits: Vec<String>,
    /// Breaking changes if any
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub breaking_changes: Vec<String>,
    /// Testing instructions for reviewers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub testing_notes: Option<String>,
    /// Additional notes or context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Formats a commit message from a `GeneratedMessage`
pub fn format_commit_message(response: &GeneratedMessage) -> String {
    let mut message = String::new();

    message.push_str(&response.title);
    message.push_str("\n\n");

    for line in textwrap::wrap(&response.message, 78) {
        message.push_str(&line);
        message.push('\n');
    }

    message
}

/// Formats a pull request description from a `GeneratedPullRequest`
pub fn format_pull_request(response: &GeneratedPullRequest) -> String {
    let mut message = String::new();

    // Summary - no word wrapping for web UI display
    let _ = writeln!(&mut message, "## Summary");
    let _ = writeln!(&mut message, "{}", response.summary);
    message.push('\n');

    // Description - no word wrapping for web UI display
    let _ = writeln!(&mut message, "## Description");
    let _ = writeln!(&mut message, "{}", response.description);
    message.push('\n');

    // Commits
    if !response.commits.is_empty() {
        let _ = writeln!(&mut message, "## Commits");
        for commit in &response.commits {
            let _ = writeln!(&mut message, "- {commit}");
        }
        message.push('\n');
    }

    // Breaking changes
    if !response.breaking_changes.is_empty() {
        let _ = writeln!(&mut message, "## Breaking Changes");
        for change in &response.breaking_changes {
            let _ = writeln!(&mut message, "- {change}");
        }
        message.push('\n');
    }

    // Testing notes - no word wrapping for web UI display
    if let Some(testing) = &response.testing_notes {
        let _ = writeln!(&mut message, "## Testing");
        let _ = writeln!(&mut message, "{testing}");
        message.push('\n');
    }

    // Additional notes - no word wrapping for web UI display
    if let Some(notes) = &response.notes {
        let _ = writeln!(&mut message, "## Notes");
        let _ = writeln!(&mut message, "{notes}");
    }

    message
}
