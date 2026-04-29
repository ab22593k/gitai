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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commits: Vec<String>,
    /// Breaking changes if any
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub breaking_changes: Vec<String>,
    /// Testing instructions for reviewers
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub testing_notes: Option<String>,
    /// Additional notes or context
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

    #[test]
    fn test_format_pull_request_full_output() {
        let pr = GeneratedPullRequest {
            title: "feat: add login flow".to_string(),
            summary: "Adds OAuth2 login.".to_string(),
            description: "Implements the full login flow.".to_string(),
            commits: vec![
                "feat: add auth module".to_string(),
                "fix: token refresh".to_string(),
            ],
            breaking_changes: vec!["Auth header format changed".to_string()],
            testing_notes: Some("Verify with test OAuth server.".to_string()),
            notes: Some("See RFC 6749.".to_string()),
        };

        let formatted = format_pull_request(&pr);

        assert!(formatted.contains("## Summary\nAdds OAuth2 login."));
        assert!(formatted.contains("## Description\nImplements the full login flow."));
        assert!(formatted.contains("## Commits\n- feat: add auth module\n- fix: token refresh"));
        assert!(formatted.contains("## Breaking Changes\n- Auth header format changed"));
        assert!(formatted.contains("## Testing\nVerify with test OAuth server."));
        assert!(formatted.contains("## Notes\nSee RFC 6749."));
    }

    #[test]
    fn test_format_pull_request_omits_empty_sections() {
        let pr = GeneratedPullRequest {
            title: "fix: typo".to_string(),
            summary: "Fixes a typo.".to_string(),
            description: "Minor fix.".to_string(),
            commits: Vec::new(),
            breaking_changes: Vec::new(),
            testing_notes: None,
            notes: None,
        };

        let formatted = format_pull_request(&pr);

        assert!(formatted.contains("## Summary"));
        assert!(formatted.contains("## Description"));
        assert!(!formatted.contains("## Commits"));
        assert!(!formatted.contains("## Breaking Changes"));
        assert!(!formatted.contains("## Testing"));
        assert!(!formatted.contains("## Notes"));
    }

    #[test]
    fn test_format_pull_request_commits_without_breaking_changes() {
        let pr = GeneratedPullRequest {
            title: "feat: batch API".to_string(),
            summary: "Adds batch endpoint.".to_string(),
            description: "New /batch route.".to_string(),
            commits: vec!["feat: batch route".to_string()],
            breaking_changes: Vec::new(),
            testing_notes: None,
            notes: None,
        };

        let formatted = format_pull_request(&pr);

        assert!(formatted.contains("## Commits\n- feat: batch route"));
        assert!(!formatted.contains("## Breaking Changes"));
        assert!(!formatted.contains("## Testing"));
        assert!(!formatted.contains("## Notes"));
    }

    #[test]
    fn test_format_pull_request_breaking_changes_without_commits() {
        let pr = GeneratedPullRequest {
            title: "refactor!: rename API".to_string(),
            summary: "Renames endpoints.".to_string(),
            description: "All /v1 routes moved to /v2.".to_string(),
            commits: Vec::new(),
            breaking_changes: vec!["All v1 endpoints removed".to_string()],
            testing_notes: Some("Run integration suite.".to_string()),
            notes: None,
        };

        let formatted = format_pull_request(&pr);

        assert!(!formatted.contains("## Commits"));
        assert!(formatted.contains("## Breaking Changes\n- All v1 endpoints removed"));
        assert!(formatted.contains("## Testing\nRun integration suite."));
        assert!(!formatted.contains("## Notes"));
    }

    #[test]
    fn test_generated_pull_request_serde_round_trip() {
        let pr = GeneratedPullRequest {
            title: "feat: serde test".to_string(),
            summary: "Tests serialization.".to_string(),
            description: "Round-trip test.".to_string(),
            commits: vec!["commit 1".to_string()],
            breaking_changes: Vec::new(),
            testing_notes: Some("Check serde.".to_string()),
            notes: None,
        };

        let json = serde_json::to_string(&pr).expect("serialization should succeed");
        assert!(
            !json.contains("breaking_changes"),
            "empty vec should be skipped"
        );
        assert!(!json.contains("\"notes\""), "None should be skipped");
        assert!(json.contains("testing_notes"), "Some should be present");

        let deserialized: GeneratedPullRequest =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(deserialized.title, pr.title);
        assert_eq!(deserialized.summary, pr.summary);
        assert_eq!(deserialized.commits, pr.commits);
        assert!(deserialized.breaking_changes.is_empty());
        assert_eq!(deserialized.testing_notes, pr.testing_notes);
        assert!(deserialized.notes.is_none());
    }
}
