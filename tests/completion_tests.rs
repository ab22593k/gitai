use gait::core::context::{ChangeType, CommitContext, RecentCommit, StagedFile};

#[cfg(test)]
mod tests {
    use super::*;
    use gait::Config;
    use gait::features::commit::prompt::{
        create_completion_system_prompt, create_completion_user_prompt,
    };

    fn create_test_config() -> Config {
        Config::default()
    }

    fn create_mock_commit_context() -> CommitContext {
        use gait::core::context::{ChangeType, CommitContext, RecentCommit, StagedFile};

        CommitContext::new(
            "main".to_string(),
            vec![RecentCommit {
                hash: "abc123".to_string(),
                message: "feat: add new feature".to_string(),
                timestamp: "2023-01-01 00:00:00".to_string(),
            }],
            vec![StagedFile {
                path: "src/main.rs".to_string(),
                change_type: ChangeType::Modified,
                diff: "- old line\n+ new line".to_string(),
                content: Some("fn main() {}".to_string()),
                content_excluded: false,
            }],
            "Test User".to_string(),
            "test@example.com".to_string(),
            vec!["feat: add feature".to_string(), "fix: bug fix".to_string()],
        )
    }

    #[test]
    fn test_completion_context_creation() {
        let context = CommitContext::new(
            "main".to_string(),
            vec![RecentCommit {
                hash: "abc123".to_string(),
                message: "feat: add new feature".to_string(),
                timestamp: "2023-01-01".to_string(),
            }],
            vec![StagedFile {
                path: "src/main.rs".to_string(),
                change_type: ChangeType::Modified,
                diff: "+ new line".to_string(),
                content: Some("fn main() {}".to_string()),
                content_excluded: false,
            }],
            "Test User".to_string(),
            "test@example.com".to_string(),
            vec!["feat: add feature".to_string(), "fix: bug fix".to_string()],
        );

        assert_eq!(context.branch, "main");
        assert_eq!(context.recent_commits.len(), 1);
        assert_eq!(context.staged_files.len(), 1);
        assert_eq!(context.author_history.len(), 2);
    }

    #[test]
    fn test_completion_user_prompt_format() {
        let context = create_mock_commit_context();
        let prefix = "feat: add user";
        let context_ratio = 0.5;

        let prompt = create_completion_user_prompt(&context, prefix, context_ratio);

        // Check that the prompt starts with task header
        assert!(
            prompt.starts_with("# TASK: Complete Commit Message"),
            "Completion prompt should start with task header"
        );

        // Check that it contains the prefix
        assert!(
            prompt.contains(&format!("**Prefix:** '{prefix}'")),
            "Should contain the prefix"
        );

        // Check that it contains context ratio
        assert!(
            prompt.contains("**Context Ratio:** 50%"),
            "Should contain context ratio"
        );

        // Check that it contains all context sections
        assert!(prompt.contains("Branch:"), "Should contain branch section");
        assert!(
            prompt.contains("Recent Commits (for changed files):"),
            "Should contain recent commits section"
        );
        assert!(
            prompt.contains("Staged Changes:"),
            "Should contain staged changes section"
        );
        assert!(
            prompt.contains("Detailed Changes:"),
            "Should contain detailed changes section"
        );
        assert!(
            prompt.contains("Author's Commit History:"),
            "Should contain author history section"
        );

        // Check that it doesn't contain the old "COMPLETE" instruction
        assert!(
            !prompt.contains("COMPLETE the commit message"),
            "Should not contain old COMPLETE instruction"
        );
    }

    #[test]
    fn test_completion_system_prompt_structure() {
        let config = create_test_config();
        let prompt = create_completion_system_prompt(&config)
            .expect("Failed to create completion system prompt");

        // Check that it defines the role correctly
        assert!(
            prompt.contains("Git Commit Message Completion Specialist"),
            "Should define completion specialist role"
        );

        // Check that it has completion rules
        assert!(
            prompt.contains("## Completion Rules"),
            "Should contain completion rules section"
        );

        // Check that it mentions starting where prefix ends
        assert!(
            prompt.contains("Begin completion exactly where the prefix ends"),
            "Should mention prefix continuation"
        );

        // Check that it mentions maintaining conventions
        assert!(
            prompt.contains("Maintain the same tone, style, and conventions"),
            "Should mention style maintenance"
        );
    }

    #[test]
    fn test_completion_context_enhancement() {
        let mut context = create_mock_commit_context();

        // Add some author history
        context.author_history = vec![
            "feat: add user authentication".to_string(),
            "fix: resolve login issue".to_string(),
            "docs: update README".to_string(),
        ];

        // Test that enhanced history is created
        let enhanced = context.get_enhanced_history(10);
        assert!(!enhanced.is_empty(), "Enhanced history should not be empty");
        assert!(
            enhanced.len() <= 10,
            "Enhanced history should be limited to max size"
        );
    }

    #[test]
    fn test_convention_detection() {
        let context = CommitContext::new(
            "main".to_string(),
            vec![],
            vec![],
            "Test User".to_string(),
            "test@example.com".to_string(),
            vec![
                "feat: add new feature".to_string(),
                "fix: resolve bug".to_string(),
                "feat: implement api".to_string(),
                "Add new component".to_string(), // imperative without colon
            ],
        );

        let conventions = context.detect_conventions();
        assert!(conventions.contains_key("feat:"));
        assert!(conventions.contains_key("fix:"));
        assert!(conventions.contains_key("imperative"));
        assert_eq!(
            *conventions.get("feat:").expect("feat: key should exist"),
            2
        );
        assert_eq!(*conventions.get("fix:").expect("fix: key should exist"), 1);
    }

    #[test]
    fn test_enhanced_history() {
        let context = CommitContext::new(
            "main".to_string(),
            vec![],
            vec![StagedFile {
                path: "auth.rs".to_string(),
                change_type: ChangeType::Modified,
                diff: "+ auth logic".to_string(),
                content: Some("fn auth() {}".to_string()),
                content_excluded: false,
            }],
            "Test User".to_string(),
            "test@example.com".to_string(),
            vec![
                "feat: add user auth".to_string(),
                "fix: login issue".to_string(),
                "docs: update api".to_string(),
                "refactor: clean code".to_string(),
            ],
        );

        let enhanced = context.get_enhanced_history(3);
        assert!(enhanced.len() <= 3);
        // Should prioritize semantically similar commits
        assert!(enhanced.iter().any(|msg| msg.contains("auth")));
    }
}
