use gitpilot::emoji::{apply_emoji, get_emoji, get_emoji_list};

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::TestAssertions;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_emoji() {
        // Test standard emoji applications
        assert_eq!(
            apply_emoji("feat: add new feature"),
            "✨ feat: add new feature"
        );
        assert_eq!(apply_emoji("fix: resolve bug"), "🐛 fix: resolve bug");
        assert_eq!(
            apply_emoji("docs: update documentation"),
            "📝 docs: update documentation"
        );
        assert_eq!(apply_emoji("style: format code"), "💄 style: format code");
        assert_eq!(
            apply_emoji("refactor: improve code structure"),
            "♻️ refactor: improve code structure"
        );
        assert_eq!(
            apply_emoji("test: add unit tests"),
            "✅ test: add unit tests"
        );
        assert_eq!(
            apply_emoji("chore: update dependencies"),
            "🔨 chore: update dependencies"
        );

        // Test edge cases
        assert_eq!(
            apply_emoji("unknown: some message"),
            "unknown: some message"
        );
        assert_eq!(apply_emoji(""), "");
        assert_eq!(apply_emoji("no_colon_here"), "no_colon_here");
    }

    #[test]
    fn test_get_emoji_list() {
        let list = get_emoji_list();

        // Use our centralized assertion for emoji validation
        TestAssertions::assert_contains_emoji(&list);

        // Additional specific checks
        assert!(list.contains("✨ - :feat: - Introduce new features"));
        assert!(list.contains("🐛 - :fix: - Fix a bug"));
        assert!(list.contains("📝 - :docs: - Add or update documentation"));
        assert!(list.contains("💄 - :style: - Add or update the UI and style files"));
        assert!(list.contains("♻️ - :refactor: - Refactor code"));
        assert!(list.contains("✅ - :test: - Add or update tests"));
        assert!(list.contains("🔨 - :chore: - Other changes that don't modify src or test files"));
    }

    #[test]
    fn test_get_emoji() {
        // Test valid emoji lookups
        assert_eq!(get_emoji("feat"), Some("✨"));
        assert_eq!(get_emoji("fix"), Some("🐛"));
        assert_eq!(get_emoji("docs"), Some("📝"));
        assert_eq!(get_emoji("style"), Some("💄"));
        assert_eq!(get_emoji("refactor"), Some("♻️"));
        assert_eq!(get_emoji("test"), Some("✅"));
        assert_eq!(get_emoji("chore"), Some("🔨"));

        // Test invalid lookup
        assert_eq!(get_emoji("unknown"), None);
    }
}
