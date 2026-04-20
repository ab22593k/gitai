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

    for line in textwrap::wrap(&response.message, 72) {
        message.push_str(&line);
        message.push('\n');
    }

    message
}

/// Formats a pull request description from a `GeneratedPullRequest`
pub fn format_pull_request(response: &GeneratedPullRequest) -> String {
    let mut message = String::new();

    // Summary - no word wrapping for web UI display
    writeln!(&mut message, "## Summary").expect("String write is infallible");
    writeln!(&mut message, "{}", response.summary).expect("String write is infallible");
    message.push('\n');

    // Description - no word wrapping for web UI display
    writeln!(&mut message, "## Description").expect("String write is infallible");
    writeln!(&mut message, "{}", response.description).expect("String write is infallible");
    message.push('\n');

    // Commits
    if !response.commits.is_empty() {
        writeln!(&mut message, "## Commits").expect("String write is infallible");
        for commit in &response.commits {
            writeln!(&mut message, "- {commit}").expect("String write is infallible");
        }
        message.push('\n');
    }

    // Breaking changes
    if !response.breaking_changes.is_empty() {
        writeln!(&mut message, "## Breaking Changes").expect("String write is infallible");
        for change in &response.breaking_changes {
            writeln!(&mut message, "- {change}").expect("String write is infallible");
        }
        message.push('\n');
    }

    // Testing notes - no word wrapping for web UI display
    if let Some(testing) = &response.testing_notes {
        writeln!(&mut message, "## Testing").expect("String write is infallible");
        writeln!(&mut message, "{testing}").expect("String write is infallible");
        message.push('\n');
    }

    // Additional notes - no word wrapping for web UI display
    if let Some(notes) = &response.notes {
        writeln!(&mut message, "## Notes").expect("String write is infallible");
        writeln!(&mut message, "{notes}").expect("String write is infallible");
    }

    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_commit_message_wraps_at_72_chars() {
        let message = GeneratedMessage {
            title: "feat: add new feature".to_string(),
            message: "This is a very long line that should be wrapped at 72 characters to comply with Git commit message conventions and best practices.".to_string(),
        };

        let formatted = format_commit_message(&message);
        let lines: Vec<&str> = formatted.lines().collect();

        // First line should be the title
        assert_eq!(lines[0], "feat: add new feature");

        // Second line should be empty (separator)
        assert_eq!(lines[1], "");

        // Body lines should be wrapped at 72 characters
        for line in &lines[2..] {
            assert!(
                line.len() <= 72,
                "Line exceeds 72 characters: '{}' (length: {})",
                line,
                line.len()
            );
        }
    }

    #[test]
    fn test_format_commit_message_preserves_short_lines() {
        let message = GeneratedMessage {
            title: "fix: short title".to_string(),
            message: "Short body.\nAnother short line.".to_string(),
        };

        let formatted = format_commit_message(&message);
        let lines: Vec<&str> = formatted.lines().collect();

        assert_eq!(lines[0], "fix: short title");
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "Short body.");
        assert_eq!(lines[3], "Another short line.");
    }

    #[test]
    fn test_format_commit_message_handles_empty_body() {
        let message = GeneratedMessage {
            title: "chore: update dependencies".to_string(),
            message: String::new(),
        };

        let formatted = format_commit_message(&message);

        // Should have title, blank line, and then empty body
        assert!(formatted.starts_with("chore: update dependencies\n\n"));
    }
}
