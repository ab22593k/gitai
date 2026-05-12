use anyhow::{Context, Result};
use chrono;
use cloy::commands::changelog::change_analyzer::AnalyzedChange;
use cloy::commands::changelog::common::generate_changes_content;
use cloy::commands::changelog::models::{
    BreakingChange, ChangeEntry, ChangeMetrics, ChangelogResponse, ChangelogType,
};
use cloy::common::DetailLevel;
use cloy::config::Config;
use cloy::git::GitRepo;
use colored::Colorize;
use log::debug;
use prompts::changelog as changelog_prompts;
use regex;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

pub struct ChangelogGenerator;

impl ChangelogGenerator {
    pub async fn generate(
        git_repo: Arc<GitRepo>,
        from: &str,
        to: &str,
        config: &Config,
        detail_level: DetailLevel,
    ) -> Result<String> {
        let changelog: ChangelogResponse = generate_changes_content::<ChangelogResponse>(
            git_repo,
            from,
            to,
            config,
            detail_level,
            system_prompt_adapter,
            user_prompt_adapter,
        )
        .await?;

        Ok(format_changelog_response(&changelog))
    }

    pub fn update_changelog_file(
        changelog_content: &str,
        changelog_path: &str,
        git_repo: &Arc<GitRepo>,
        to_ref: &str,
        version_name: Option<String>,
    ) -> Result<()> {
        let path = Path::new(changelog_path);
        let commit_date = get_commit_date(git_repo, to_ref);
        let clean_content = prepare_version_content(changelog_content, &commit_date, version_name);
        let updated_content = merge_with_existing(path, &clean_content)?;
        write_changelog_file(path, &updated_content, changelog_path)?;
        Ok(())
    }
}

fn system_prompt_adapter(config: &Config) -> String {
    let schema = schemars::schema_for!(ChangelogResponse);
    let schema_str = serde_json::to_string_pretty(&schema).unwrap_or_else(|_| String::from("{}"));
    let instructions = cloy::common::get_combined_instructions(config);
    changelog_prompts::create_changelog_system_prompt(&instructions, &schema_str)
}

fn user_prompt_adapter(
    changes: &[AnalyzedChange],
    total_metrics: &ChangeMetrics,
    detail_level: DetailLevel,
    from: &str,
    to: &str,
    readme_summary: Option<&str>,
) -> String {
    let mut metrics_buf = String::new();
    writeln!(metrics_buf, "Overall Changes:").ok();
    writeln!(
        metrics_buf,
        "Total commits: {}",
        total_metrics.total_commits
    )
    .ok();
    writeln!(
        metrics_buf,
        "Files changed: {}",
        total_metrics.files_changed
    )
    .ok();
    writeln!(
        metrics_buf,
        "Total lines changed: {}",
        total_metrics.total_lines_changed
    )
    .ok();
    writeln!(metrics_buf, "Insertions: {}", total_metrics.insertions).ok();
    writeln!(metrics_buf, "Deletions: {}\n", total_metrics.deletions).ok();

    let mut changes_buf = String::new();
    for change in changes {
        writeln!(changes_buf, "Commit: {}", change.commit_hash).ok();
        writeln!(changes_buf, "Message: {}", change.commit_message).ok();
        writeln!(changes_buf, "Type: {:?}", change.change_type).ok();
        writeln!(
            changes_buf,
            "Breaking Change: {}",
            change.is_breaking_change
        )
        .ok();
        writeln!(
            changes_buf,
            "Associated Issues: {}",
            change.associated_issues.join(", ")
        )
        .ok();
        if let Some(pr) = &change.pull_request {
            writeln!(changes_buf, "Pull Request: {pr}").ok();
        }
        writeln!(changes_buf, "Impact score: {:.2}", change.impact_score).ok();

        match detail_level {
            DetailLevel::Minimal => {}
            DetailLevel::Standard | DetailLevel::Detailed => {
                changes_buf.push_str("File changes:\n");
                for file_change in &change.file_changes {
                    writeln!(
                        changes_buf,
                        "  - {} ({:?})",
                        file_change.new_path, file_change.change_type
                    )
                    .ok();
                    if detail_level == DetailLevel::Detailed {
                        for analysis in &file_change.analysis {
                            writeln!(changes_buf, "    * {analysis}").ok();
                        }
                    }
                }
            }
        }
        changes_buf.push('\n');
    }

    let detail_req = match detail_level {
        DetailLevel::Minimal => {
            "EXIGENCY: Extreme technical brevity. Focus only on architectural shifts."
        }
        DetailLevel::Standard => {
            "EXIGENCY: Provide a balanced technical narrative of all significant changes."
        }
        DetailLevel::Detailed => {
            "EXIGENCY: Exhaustive technical documentation. Explain the 'Why' for major file changes."
        }
    };

    changelog_prompts::create_changelog_user_prompt(
        from,
        to,
        &metrics_buf,
        &changes_buf,
        readme_summary,
        detail_req,
    )
}

fn get_commit_date(git_repo: &Arc<GitRepo>, to_ref: &str) -> String {
    match git_repo.get_commit_date(to_ref) {
        Ok(date) => {
            debug!("Got commit date for {to_ref}: {date}");
            date
        }
        Err(e) => {
            debug!("Failed to get commit date for {to_ref}: {e}");
            chrono::Local::now().format("%Y-%m-%d").to_string()
        }
    }
}

fn prepare_version_content(
    changelog_content: &str,
    commit_date: &str,
    version_name: Option<String>,
) -> String {
    let stripped = strip_ansi_codes(changelog_content);
    let clean = clean_separator(&stripped);
    let version_content = extract_version_section(&clean);
    let with_version = apply_version_override(&version_content, version_name);
    ensure_date_in_content(&with_version, commit_date)
}

fn clean_separator(content: &str) -> String {
    if content.starts_with("━") || content.starts_with('-') {
        if let Some(pos) = content.find('\n') {
            content[pos + 1..].to_string()
        } else {
            content.to_string()
        }
    } else {
        content.to_string()
    }
}

fn extract_version_section(content: &str) -> String {
    if let Some(parts) = content.split("## [").collect::<Vec<_>>().get(1) {
        format!("## [{parts}")
    } else {
        content.to_string()
    }
}

fn apply_version_override(content: &str, version_name: Option<String>) -> String {
    match version_name {
        Some(version) if content.contains("## [") => {
            let re = regex::Regex::new(r"## \[([^\]]+)\]").expect("Failed to compile regex");
            let result = re.replace(content, format!("## [{version}]")).to_string();
            debug!("Replaced version with user-provided version: {version}");
            result
        }
        Some(_) => {
            debug!("Could not find version header to replace in changelog content");
            content.to_string()
        }
        None => content.to_string(),
    }
}

fn ensure_date_in_content(content: &str, commit_date: &str) -> String {
    if content.contains(" - \n") {
        let result = content.replace(" - \n", &format!(" - {commit_date}\n"));
        debug!("Replaced empty date with commit date: {commit_date}");
        result
    } else if content.contains("] - ") && !content.contains("] - 20") {
        let parts: Vec<&str> = content.splitn(2, "] - ").collect();
        if parts.len() == 2 {
            debug!("Added commit date after dash: {commit_date}");
            format!(
                "{}] - {}\n{}",
                parts[0],
                commit_date,
                parts[1].trim_start_matches(['\n', ' '])
            )
        } else {
            content.to_string()
        }
    } else if !content.contains("] - ") {
        add_date_to_version_line(content, commit_date)
    } else {
        content.to_string()
    }
}

fn add_date_to_version_line(content: &str, commit_date: &str) -> String {
    let line_end = content.find('\n').unwrap_or(content.len());
    let version_line = &content[..line_end];

    if version_line.contains("## [") && version_line.contains(']') {
        let bracket_pos = version_line
            .rfind(']')
            .expect("Failed to find closing bracket");
        debug!("Added date to version line: {commit_date}");
        format!(
            "{} - {}{}",
            &content[..=bracket_pos],
            commit_date,
            &content[bracket_pos + 1..]
        )
    } else {
        content.to_string()
    }
}

fn merge_with_existing(path: &Path, new_content: &str) -> Result<String> {
    let default_header = "# Changelog\n\nAll notable changes to this project will be documented in this file.\n\nThe format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),\nand this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n";
    let separator = "\n<!-- -------------------------------------------------------------- -->\n\n";
    let version_with_separator = format!("{new_content}{separator}");

    if path.exists() {
        let existing_content = fs::read_to_string(path)?;
        if existing_content.contains("# Changelog") && existing_content.contains("Keep a Changelog")
        {
            Ok(merge_with_keep_a_changelog(
                &existing_content,
                &version_with_separator,
            ))
        } else {
            Ok(format!("{default_header}{version_with_separator}"))
        }
    } else {
        Ok(format!("{default_header}{version_with_separator}"))
    }
}

fn merge_with_keep_a_changelog(existing: &str, new_content: &str) -> String {
    let parts: Vec<&str> = existing.split("## [").collect();
    let header = parts[0];
    if parts.len() > 1 {
        let existing_versions = parts[1..].join("## [");
        format!("{header}{new_content}## [{existing_versions}")
    } else {
        format!("{existing}{new_content}")
    }
}

fn write_changelog_file(path: &Path, content: &str, changelog_path: &str) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create changelog file: {changelog_path}"))?;
    file.write_all(content.as_bytes())
        .with_context(|| format!("Failed to write to changelog file: {changelog_path}"))?;
    Ok(())
}

fn strip_ansi_codes(s: &str) -> String {
    let re = regex::Regex::new(r"\x1B\[([0-9]{1,2}(;[0-9]{1,2})*)?[m|K]")
        .expect("Failed to compile ANSI escape code regex");
    re.replace_all(s, "").to_string()
}

fn format_changelog_response(response: &ChangelogResponse) -> String {
    let mut formatted = String::new();

    formatted.push_str(&"# Changelog\n\n".bright_cyan().bold().to_string());
    formatted.push_str("All notable changes to this project will be documented in this file.\n\n");
    formatted.push_str(
        "The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),\n",
    );
    formatted.push_str("and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n");

    let version = response
        .version
        .clone()
        .unwrap_or_else(|| "Unreleased".to_string());

    write!(formatted, "## [{}] - \n\n", version.bright_green().bold())
        .expect("writing to string should never fail");

    let ordered_types = [
        ChangelogType::Added,
        ChangelogType::Changed,
        ChangelogType::Fixed,
        ChangelogType::Removed,
        ChangelogType::Deprecated,
        ChangelogType::Security,
    ];

    for change_type in &ordered_types {
        if let Some(entries) = response.sections.get(change_type)
            && !entries.is_empty()
        {
            formatted.push_str(&format_change_type(change_type));
            for entry in entries {
                formatted.push_str(&format_change_entry(entry));
            }
            formatted.push('\n');
        }
    }

    if !response.breaking_changes.is_empty() {
        formatted.push_str(
            &"### ⚠️ Breaking Changes\n\n"
                .bright_red()
                .bold()
                .to_string(),
        );
        for breaking_change in &response.breaking_changes {
            formatted.push_str(&format_breaking_change(breaking_change));
        }
        formatted.push('\n');
    }

    formatted.push_str(&"### 📊 Metrics\n\n".bright_magenta().bold().to_string());
    formatted.push_str(&format_metrics(&response.metrics));

    formatted
}

fn format_change_type(change_type: &ChangelogType) -> String {
    let (emoji, text) = match change_type {
        ChangelogType::Added => ("✨", "Added"),
        ChangelogType::Changed => ("🔄", "Changed"),
        ChangelogType::Deprecated => ("⚠️", "Deprecated"),
        ChangelogType::Removed => ("🗑️", "Removed"),
        ChangelogType::Fixed => ("🐛", "Fixed"),
        ChangelogType::Security => ("🔒", "Security"),
    };
    format!("### {} {}\n\n", emoji, text.bright_blue().bold())
}

fn format_change_entry(entry: &ChangeEntry) -> String {
    let mut formatted = format!("- {}", entry.description);

    if !entry.associated_issues.is_empty() {
        write!(
            formatted,
            " ({})",
            entry.associated_issues.join(", ").yellow()
        )
        .expect("writing to string should never fail");
    }

    if let Some(pr) = &entry.pull_request {
        write!(formatted, " [{}]", pr.bright_purple())
            .expect("writing to string should never fail");
    }

    writeln!(formatted, " ({})", entry.commit_hashes.join(", ").dimmed())
        .expect("writing to string should never fail");

    formatted
}

fn format_breaking_change(breaking_change: &BreakingChange) -> String {
    format!(
        "- {} ({})\n",
        breaking_change.description,
        breaking_change.commit_hash.dimmed()
    )
}

fn format_metrics(metrics: &ChangeMetrics) -> String {
    format!(
        "- Total Commits: {}\n- Files Changed: {}\n- Insertions: {}\n- Deletions: {}\n",
        metrics.total_commits.to_string().green(),
        metrics.files_changed.to_string().yellow(),
        metrics.insertions.to_string().green(),
        metrics.deletions.to_string().red()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloy::commands::changelog::models::{
        BreakingChange, ChangeEntry, ChangeMetrics, ChangelogResponse, ChangelogType,
    };
    use std::collections::HashMap;

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
        // Verify ordering: 1.0.0 appears before 0.1.0
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
        // Non-keep-a-changelog format: still writes the default header + new content
        assert!(result.contains("# Changelog"));
        assert!(result.contains("Keep a Changelog"));
        // The old content is not preserved in this case
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
        // Added before Fixed per ordered_types
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
        // All section headers should be absent
        assert!(
            !result.contains("✨"),
            "empty Added section should be omitted"
        );
        assert!(
            !result.contains("🐛"),
            "empty Fixed section should be omitted"
        );
        // Breaking changes section should be absent
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
        let saved = std::fs::read_to_string(&changelog_path).expect("read file");
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

        // Write existing keep-a-changelog file
        std::fs::write(
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

        let saved = std::fs::read_to_string(&changelog_path).expect("read file");
        // Both versions present
        assert!(saved.contains("1.0.0"));
        assert!(saved.contains("0.1.0"));
        assert!(saved.contains("new feature"));
        assert!(saved.contains("old feature"));
        // New version before old version
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

        let saved = std::fs::read_to_string(&changelog_path).expect("read file");
        assert!(saved.contains("2.0.0"), "should use overridden version");
        assert!(
            !saved.contains("Unreleased"),
            "should not contain original version"
        );
    }
}
