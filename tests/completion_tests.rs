use gitai::core::context::{ChangeType, CommitContext, RecentCommit, StagedFile};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_context_creation() {
        let context = CommitContext::new(
            "main".to_string(),
            vec![RecentCommit {
                hash: "abc123".to_string(),
                message: "feat: add new feature".to_string(),
                author: "test@example.com".to_string(),
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
    fn test_semantic_similarity_scoring() {
        let context = CommitContext::new(
            "main".to_string(),
            vec![],
            vec![StagedFile {
                path: "user_authentication.rs".to_string(),
                change_type: ChangeType::Modified,
                diff: "+ authentication logic".to_string(),
                content: Some("fn authenticate() {}".to_string()),
                content_excluded: false,
            }],
            "Test User".to_string(),
            "test@example.com".to_string(),
            vec![
                "feat: add user authentication".to_string(),
                "fix: resolve login issue".to_string(),
                "docs: update readme".to_string(),
            ],
        );

        let similar = context.get_similar_history(5);
        assert!(!similar.is_empty());
        // Should find the authentication-related commit as most similar
        assert!(similar[0].contains("authentication"));
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

    #[test]
    fn test_keyword_extraction() {
        let context = CommitContext::new(
            "main".to_string(),
            vec![],
            vec![
                StagedFile {
                    path: "user_authentication_service.rs".to_string(),
                    change_type: ChangeType::Modified,
                    diff: "+ authentication service".to_string(),
                    content: Some(
                        "class UserAuthenticationService { authenticate() {} }".to_string(),
                    ),
                    content_excluded: false,
                },
                StagedFile {
                    path: "login_component.tsx".to_string(),
                    change_type: ChangeType::Added,
                    diff: "+ login component".to_string(),
                    content: Some("function LoginComponent() {}".to_string()),
                    content_excluded: false,
                },
            ],
            "Test User".to_string(),
            "test@example.com".to_string(),
            vec![],
        );

        // Test that keywords are extracted from file names and content
        // This is tested indirectly through the similarity scoring
        let similar = context.get_similar_history(5);
        // Should work without panicking even with empty history
        assert_eq!(similar.len(), 0);
    }
}
