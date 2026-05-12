use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use changelog::change_log::{
    ChangelogGenerator, add_date_to_version_line, apply_version_override, clean_separator,
    ensure_date_in_content, extract_version_section, format_breaking_change, format_change_entry,
    format_change_type, format_changelog_response, format_metrics, merge_with_existing,
    merge_with_keep_a_changelog, prepare_version_content, strip_ansi_codes,
};
use cloy::commands::changelog::models::{
    BreakingChange, ChangeEntry, ChangeMetrics, ChangelogResponse, ChangelogType,
};

fn sample_changelog_response() -> ChangelogResponse {
    ChangelogResponse {
        version: Some("1.0.0".to_string()),
        release_date: Some("2024-01-15".to_string()),
        sections: HashMap::from([
            (
                ChangelogType::Added,
                vec![
                    ChangeEntry {
                        description: "New feature X".to_string(),
                        commit_hashes: vec!["abc123".to_string()],
                        associated_issues: vec!["#42".to_string()],
                        pull_request: Some("#123".to_string()),
                    },
                    ChangeEntry {
                        description: "Another feature".to_string(),
                        commit_hashes: vec!["def456".to_string()],
                        associated_issues: vec![],
                        pull_request: None,
                    },
                ],
            ),
            (
                ChangelogType::Fixed,
                vec![ChangeEntry {
                    description: "Bug fix Y".to_string(),
                    commit_hashes: vec!["ghi789".to_string()],
                    associated_issues: vec![],
                    pull_request: None,
                }],
            ),
        ]),
        breaking_changes: vec![BreakingChange {
            description: "Removed old API".to_string(),
            commit_hash: "jkl012".to_string(),
        }],
        metrics: ChangeMetrics {
            total_commits: 10,
            files_changed: 25,
            insertions: 500,
            deletions: 100,
            total_lines_changed: 600,
        },
    }
}

// -- strip_ansi_codes --

#[test]
fn test_strip_ansi_codes_removes_basic() {
    assert_eq!(strip_ansi_codes("\x1b[31mhello\x1b[0m"), "hello");
}

#[test]
fn test_strip_ansi_codes_multi_param() {
    assert_eq!(strip_ansi_codes("\x1b[1;31mhello\x1b[0m"), "hello");
}

#[test]
fn test_strip_ansi_codes_none() {
    assert_eq!(strip_ansi_codes("hello world"), "hello world");
}

#[test]
fn test_strip_ansi_codes_empty() {
    assert_eq!(strip_ansi_codes(""), "");
}

#[test]
fn test_strip_ansi_codes_erase_line() {
    assert_eq!(strip_ansi_codes("\x1b[Kclear"), "clear");
}

// -- clean_separator --

#[test]
fn test_clean_separator_unicode() {
    assert_eq!(clean_separator("━line1\ncontent"), "content");
}

#[test]
fn test_clean_separator_dash() {
    assert_eq!(clean_separator("----\ncontent"), "content");
}

#[test]
fn test_clean_separator_none() {
    assert_eq!(clean_separator("content"), "content");
}

#[test]
fn test_clean_separator_empty() {
    assert_eq!(clean_separator(""), "");
}

#[test]
fn test_clean_separator_sep_only_no_newline() {
    assert_eq!(clean_separator("━"), "━");
}

// -- extract_version_section --

#[test]
fn test_extract_version_section_found() {
    let result = extract_version_section("prefix\n## [1.0.0] - 2024\ndetails");
    assert_eq!(result, "## [1.0.0] - 2024\ndetails");
}

#[test]
fn test_extract_version_section_not_found() {
    let result = extract_version_section("no version header here");
    assert_eq!(result, "no version header here");
}

#[test]
fn test_extract_version_section_empty() {
    assert_eq!(extract_version_section(""), "");
}

// -- apply_version_override --

#[test]
fn test_apply_version_override_replaces() {
    let result = apply_version_override("## [Unreleased] - \ncontent", Some("1.0.0".into()));
    assert_eq!(result, "## [1.0.0] - \ncontent");
}

#[test]
fn test_apply_version_override_no_header() {
    let input = "plain text";
    let result = apply_version_override(input, Some("1.0.0".into()));
    assert_eq!(result, "plain text");
}

#[test]
fn test_apply_version_override_none() {
    let input = "## [Unreleased] - \ncontent";
    let result = apply_version_override(input, None);
    assert_eq!(result, "## [Unreleased] - \ncontent");
}

// -- ensure_date_in_content --

#[test]
fn test_ensure_date_in_content_empty_placeholder() {
    let result = ensure_date_in_content("## [1.0.0] - \ncontent", "2024-06-01");
    assert_eq!(result, "## [1.0.0] - 2024-06-01\ncontent");
}

#[test]
fn test_ensure_date_in_content_no_date_yet() {
    let result = ensure_date_in_content("## [1.0.0] - ", "2024-06-01");
    assert!(result.contains("2024-06-01"));
}

#[test]
fn test_ensure_date_in_content_already_has_date() {
    let input = "## [1.0.0] - 2024-01-15\ncontent";
    let result = ensure_date_in_content(input, "2024-06-01");
    assert_eq!(result, input);
}

#[test]
fn test_ensure_date_in_content_no_dash_uses_add_date() {
    let result = ensure_date_in_content("## [1.0.0]\ncontent", "2024-06-01");
    assert_eq!(result, "## [1.0.0] - 2024-06-01\ncontent");
}

// -- add_date_to_version_line --

#[test]
fn test_add_date_to_version_line_normal() {
    let result = add_date_to_version_line("## [1.0.0]\ncontent", "2024-06-01");
    assert_eq!(result, "## [1.0.0] - 2024-06-01\ncontent");
}

#[test]
fn test_add_date_to_version_line_no_bracket() {
    let input = "no brackets\ncontent";
    let result = add_date_to_version_line(input, "2024-06-01");
    assert_eq!(result, input);
}

// -- merge_with_keep_a_changelog --

#[test]
fn test_merge_with_keep_a_changelog_inserts_before_existing() {
    let existing = "# Changelog\n\n## [0.1.0] - 2024\n### Added\n- old stuff\n";
    let new_content = "## [1.0.0] - 2024-06-01\n### Added\n- new stuff\n<!-- sep -->\n";
    let result = merge_with_keep_a_changelog(existing, new_content);
    assert_eq!(
        result,
        "# Changelog\n\n## [1.0.0] - 2024-06-01\n### Added\n- new stuff\n<!-- sep -->\n## [0.1.0] - 2024\n### Added\n- old stuff\n"
    );
}

#[test]
fn test_merge_with_keep_a_changelog_no_existing_versions() {
    let existing = "# Changelog\n\n";
    let new_content = "## [1.0.0] - 2024-06-01\n<!-- sep -->\n";
    let result = merge_with_keep_a_changelog(existing, new_content);
    assert_eq!(
        result,
        "# Changelog\n\n## [1.0.0] - 2024-06-01\n<!-- sep -->\n"
    );
}

// -- merge_with_existing (uses tempfile) --

#[test]
fn test_merge_with_existing_no_file() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let path = dir.path().join("CHANGELOG.md");
    let new_content = "## [1.0.0] - 2024-06-01\n### Added\n- feature\n";
    let result = merge_with_existing(&path, new_content).expect("merge should succeed");
    assert!(result.contains("# Changelog"));
    assert!(result.contains("Keep a Changelog"));
    assert!(result.contains("1.0.0"));
    assert!(result.contains("feature"));
}

#[test]
fn test_merge_with_existing_keep_a_changelog_format() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let path = dir.path().join("CHANGELOG.md");
    std::fs::write(
        &path,
        "# Changelog\n\nAll notable changes\n\nThe format is based on [Keep a Changelog](https://keepachangelog.com/)\n\n## [0.1.0] - 2024\n### Added\n- old\n",
    )
    .expect("write existing");
    let new_content = "## [1.0.0] - 2024-06-01\n<!-- sep -->\n";
    let result = merge_with_existing(&path, new_content).expect("merge should succeed");
    assert!(result.contains("1.0.0"));
    assert!(result.contains("0.1.0"));
    assert!(result.contains("old"));
    let pos_new = result.find("1.0.0").expect("new version");
    let pos_old = result.find("0.1.0").expect("old version");
    assert!(pos_new < pos_old, "new version should come before old");
}

#[test]
fn test_merge_with_existing_non_keep_a_changelog_overwrites() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let path = dir.path().join("CHANGELOG.md");
    std::fs::write(&path, "some random format content").expect("write existing");
    let new_content = "## [1.0.0] - 2024-06-01\n";
    let result = merge_with_existing(&path, new_content).expect("merge should succeed");
    assert!(result.contains("# Changelog"));
    assert!(result.contains("Keep a Changelog"));
    assert!(!result.contains("some random format"));
}

// -- format_change_type --

#[test]
fn test_format_change_type_all_variants() {
    for (change_type, emoji) in [
        (ChangelogType::Added, "✨"),
        (ChangelogType::Changed, "🔄"),
        (ChangelogType::Deprecated, "⚠️"),
        (ChangelogType::Removed, "🗑️"),
        (ChangelogType::Fixed, "🐛"),
        (ChangelogType::Security, "🔒"),
    ] {
        let result = format_change_type(&change_type);
        assert!(result.contains(emoji), "missing emoji for {change_type:?}");
        assert!(
            result.contains("###"),
            "missing header marker for {change_type:?}"
        );
    }
}

// -- format_change_entry --

#[test]
fn test_format_change_entry_minimal() {
    let entry = ChangeEntry {
        description: "Simple fix".to_string(),
        commit_hashes: vec!["abc".to_string()],
        associated_issues: vec![],
        pull_request: None,
    };
    let result = format_change_entry(&entry);
    assert!(result.contains("Simple fix"));
    assert!(result.contains("abc"));
    assert!(!result.contains("#42"), "no issues should appear");
    assert!(!result.contains("PR-42"), "no PR should appear");
}

#[test]
fn test_format_change_entry_full() {
    let entry = ChangeEntry {
        description: "Complex feature".to_string(),
        commit_hashes: vec!["def".to_string(), "ghi".to_string()],
        associated_issues: vec!["#1".to_string(), "#2".to_string()],
        pull_request: Some("PR-42".to_string()),
    };
    let result = format_change_entry(&entry);
    assert!(result.contains("Complex feature"));
    assert!(result.contains("#1"));
    assert!(result.contains("#2"));
    assert!(result.contains("PR-42"));
    assert!(result.contains("def"));
    assert!(result.contains("ghi"));
}

// -- format_breaking_change --

#[test]
fn test_format_breaking_change() {
    let bc = BreakingChange {
        description: "API removed".to_string(),
        commit_hash: "deadbeef".to_string(),
    };
    let result = format_breaking_change(&bc);
    assert!(result.contains("API removed"));
    assert!(result.contains("deadbeef"));
}

// -- format_metrics --

#[test]
fn test_format_metrics_values() {
    let m = ChangeMetrics {
        total_commits: 10,
        files_changed: 25,
        insertions: 500,
        deletions: 100,
        total_lines_changed: 600,
    };
    let result = format_metrics(&m);
    assert!(result.contains("Total Commits:"));
    assert!(result.contains("Files Changed:"));
    assert!(result.contains("Insertions:"));
    assert!(result.contains("Deletions:"));
    assert!(result.contains("10"));
    assert!(result.contains("25"));
    assert!(result.contains("500"));
    assert!(result.contains("100"));
}

#[test]
fn test_format_metrics_zero() {
    let m = ChangeMetrics {
        total_commits: 0,
        files_changed: 0,
        insertions: 0,
        deletions: 0,
        total_lines_changed: 0,
    };
    let result = format_metrics(&m);
    assert!(result.contains('0'));
}

// -- format_changelog_response --

#[test]
fn test_format_changelog_response_contains_version() {
    let response = sample_changelog_response();
    let result = format_changelog_response(&response);
    assert!(result.contains("Changelog"));
    assert!(result.contains("1.0.0"));
}

#[test]
fn test_format_changelog_response_contains_sections() {
    let response = sample_changelog_response();
    let result = format_changelog_response(&response);
    assert!(result.contains("New feature X"));
    assert!(result.contains("Another feature"));
    assert!(result.contains("Bug fix Y"));
    assert!(result.contains("#42"));
    assert!(result.contains("#123"));
    assert!(result.contains("abc123"));
    assert!(result.contains("def456"));
    assert!(result.contains("ghi789"));
}

#[test]
fn test_format_changelog_response_contains_breaking_changes() {
    let response = sample_changelog_response();
    let result = format_changelog_response(&response);
    assert!(result.contains("Removed old API"));
    assert!(result.contains("jkl012"));
}

#[test]
fn test_format_changelog_response_contains_metrics() {
    let response = sample_changelog_response();
    let result = format_changelog_response(&response);
    assert!(result.contains("Metrics"));
    assert!(result.contains("10"));
    assert!(result.contains("25"));
    assert!(result.contains("500"));
    assert!(result.contains("100"));
}

#[test]
fn test_format_changelog_response_section_order() {
    let response = sample_changelog_response();
    let result = format_changelog_response(&response);
    let added = result.find("✨").expect("Added section");
    let fixed = result.find("🐛").expect("Fixed section");
    assert!(added < fixed, "Added section should come before Fixed");
}

#[test]
fn test_format_changelog_response_unreleased_when_no_version() {
    let mut response = sample_changelog_response();
    response.version = None;
    let result = format_changelog_response(&response);
    assert!(result.contains("Unreleased"));
}

#[test]
fn test_format_changelog_response_empty_sections_omitted() {
    let response = ChangelogResponse {
        version: Some("1.0.0".to_string()),
        release_date: Some("2024-01-15".to_string()),
        sections: HashMap::new(),
        breaking_changes: vec![],
        metrics: ChangeMetrics {
            total_commits: 0,
            files_changed: 0,
            insertions: 0,
            deletions: 0,
            total_lines_changed: 0,
        },
    };
    let result = format_changelog_response(&response);
    assert!(
        !result.contains("✨"),
        "empty Added section should be omitted"
    );
    assert!(
        !result.contains("🐛"),
        "empty Fixed section should be omitted"
    );
    assert!(
        !result.contains("Breaking"),
        "empty breaking changes should be omitted"
    );
}

// -- prepare_version_content --

#[test]
fn test_prepare_version_content_strips_and_cleans() {
    let input = "\x1b[31m━\x1b[0m\n## [Unreleased] - \ncontent";
    let result = prepare_version_content(input, "2024-06-01", None);
    assert!(!result.contains('\x1b'), "ANSI codes should be stripped");
    assert!(result.contains("Unreleased"));
    assert!(result.contains("2024-06-01"));
}

#[test]
fn test_prepare_version_content_with_version_override() {
    let input = "━\n## [Unreleased] - \ncontent";
    let result = prepare_version_content(input, "2024-06-01", Some("2.0.0".into()));
    assert!(result.contains("2.0.0"));
    assert!(!result.contains("Unreleased"));
    assert!(result.contains("2024-06-01"));
}

// -- ChangelogGenerator::update_changelog_file (needs tempfile + git repo) --

#[test]
fn test_update_changelog_file_creates_new_file() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let repo_path = dir.path().join("repo");
    let changelog_path = dir.path().join("CHANGELOG.md");
    std::fs::create_dir(&repo_path).expect("create repo dir");
    let repo = git2::Repository::init(&repo_path).expect("init repo");
    let mut git_config = repo.config().expect("config");
    git_config.set_str("user.name", "Test").ok();
    git_config.set_str("user.email", "t@t.com").ok();

    let git_repo = Arc::new(cloy::git::GitRepo::new(&repo_path).expect("open GitRepo"));
    let content = "## [1.0.0] - \n### Added\n- feature\n";

    ChangelogGenerator::update_changelog_file(
        content,
        changelog_path.to_str().expect("path"),
        &git_repo,
        "HEAD",
        None,
    )
    .expect("update_changelog_file should succeed");

    assert!(changelog_path.exists(), "file should be created");
    let saved = fs::read_to_string(&changelog_path).expect("read file");
    assert!(saved.contains("1.0.0"), "should contain version");
    assert!(saved.contains("feature"), "should contain content");
}

#[test]
fn test_update_changelog_file_merges_with_existing() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let repo_path = dir.path().join("repo");
    let changelog_path = dir.path().join("CHANGELOG.md");
    std::fs::create_dir(&repo_path).expect("create repo dir");
    let repo = git2::Repository::init(&repo_path).expect("init repo");
    let mut git_config = repo.config().expect("config");
    git_config.set_str("user.name", "Test").ok();
    git_config.set_str("user.email", "t@t.com").ok();

    fs::write(
        &changelog_path,
        "# Changelog\n\nAll notable changes\n\nThe format is based on [Keep a Changelog](https://keepachangelog.com/)\n\n## [0.1.0] - 2024\n### Added\n- old feature\n",
    )
    .expect("write existing");

    let git_repo = Arc::new(cloy::git::GitRepo::new(&repo_path).expect("open GitRepo"));
    let content = "## [1.0.0] - \n### Added\n- new feature\n";

    ChangelogGenerator::update_changelog_file(
        content,
        changelog_path.to_str().expect("path"),
        &git_repo,
        "HEAD",
        None,
    )
    .expect("update_changelog_file should succeed");

    let saved = fs::read_to_string(&changelog_path).expect("read file");
    assert!(saved.contains("1.0.0"));
    assert!(saved.contains("0.1.0"));
    assert!(saved.contains("new feature"));
    assert!(saved.contains("old feature"));
    let pos_new = saved.find("1.0.0").expect("new version");
    let pos_old = saved.find("0.1.0").expect("old version");
    assert!(pos_new < pos_old, "new version should precede old");
}

#[test]
fn test_update_changelog_file_with_version_override() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let repo_path = dir.path().join("repo");
    let changelog_path = dir.path().join("CHANGELOG.md");
    std::fs::create_dir(&repo_path).expect("create repo dir");
    git2::Repository::init(&repo_path).expect("init repo");

    let git_repo = Arc::new(cloy::git::GitRepo::new(&repo_path).expect("open GitRepo"));
    let content = "## [Unreleased] - \n### Added\n- feature\n";

    ChangelogGenerator::update_changelog_file(
        content,
        changelog_path.to_str().expect("path"),
        &git_repo,
        "HEAD",
        Some("2.0.0".into()),
    )
    .expect("update_changelog_file should succeed");

    let saved = fs::read_to_string(&changelog_path).expect("read file");
    assert!(saved.contains("2.0.0"), "should use overridden version");
    assert!(
        !saved.contains("Unreleased"),
        "should not contain original version"
    );
}
