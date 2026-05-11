use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct GeneratedPullRequest {
    pub title: String,
    pub summary: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commits: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub breaking_changes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub testing_notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

pub fn format_pull_request(response: &GeneratedPullRequest) -> String {
    let mut message = String::new();

    writeln!(&mut message, "## Summary").expect("String write is infallible");
    writeln!(&mut message, "{}", response.summary).expect("String write is infallible");
    message.push('\n');

    writeln!(&mut message, "## Description").expect("String write is infallible");
    writeln!(&mut message, "{}", response.description).expect("String write is infallible");
    message.push('\n');

    if !response.commits.is_empty() {
        writeln!(&mut message, "## Commits").expect("String write is infallible");
        for commit in &response.commits {
            writeln!(&mut message, "- {commit}").expect("String write is infallible");
        }
        message.push('\n');
    }

    if !response.breaking_changes.is_empty() {
        writeln!(&mut message, "## Breaking Changes").expect("String write is infallible");
        for change in &response.breaking_changes {
            writeln!(&mut message, "- {change}").expect("String write is infallible");
        }
        message.push('\n');
    }

    if let Some(testing) = &response.testing_notes {
        writeln!(&mut message, "## Testing").expect("String write is infallible");
        writeln!(&mut message, "{testing}").expect("String write is infallible");
        message.push('\n');
    }

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
