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

#[doc(hidden)]
pub fn prepare_version_content(
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

#[doc(hidden)]
pub fn clean_separator(content: &str) -> String {
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

#[doc(hidden)]
pub fn extract_version_section(content: &str) -> String {
    if let Some(parts) = content.split("## [").collect::<Vec<_>>().get(1) {
        format!("## [{parts}")
    } else {
        content.to_string()
    }
}

#[doc(hidden)]
pub fn apply_version_override(content: &str, version_name: Option<String>) -> String {
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

#[doc(hidden)]
pub fn ensure_date_in_content(content: &str, commit_date: &str) -> String {
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

#[doc(hidden)]
pub fn add_date_to_version_line(content: &str, commit_date: &str) -> String {
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

#[doc(hidden)]
pub fn merge_with_existing(path: &Path, new_content: &str) -> Result<String> {
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

#[doc(hidden)]
pub fn merge_with_keep_a_changelog(existing: &str, new_content: &str) -> String {
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

#[doc(hidden)]
pub fn strip_ansi_codes(s: &str) -> String {
    let re = regex::Regex::new(r"\x1B\[([0-9]{1,2}(;[0-9]{1,2})*)?[m|K]")
        .expect("Failed to compile ANSI escape code regex");
    re.replace_all(s, "").to_string()
}

#[doc(hidden)]
pub fn format_changelog_response(response: &ChangelogResponse) -> String {
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

#[doc(hidden)]
pub fn format_change_type(change_type: &ChangelogType) -> String {
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

#[doc(hidden)]
pub fn format_change_entry(entry: &ChangeEntry) -> String {
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

#[doc(hidden)]
pub fn format_breaking_change(breaking_change: &BreakingChange) -> String {
    format!(
        "- {} ({})\n",
        breaking_change.description,
        breaking_change.commit_hash.dimmed()
    )
}

#[doc(hidden)]
pub fn format_metrics(metrics: &ChangeMetrics) -> String {
    format!(
        "- Total Commits: {}\n- Files Changed: {}\n- Insertions: {}\n- Deletions: {}\n",
        metrics.total_commits.to_string().green(),
        metrics.files_changed.to_string().yellow(),
        metrics.insertions.to_string().green(),
        metrics.deletions.to_string().red()
    )
}
