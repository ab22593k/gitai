use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use textwrap;

/// Model for commit message generation results
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct GeneratedMessage {
    /// Commit message title/subject line
    pub title: String,
    /// Detailed commit message body
    pub message: String,
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
