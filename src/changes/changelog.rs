use super::common::generate_changes_content;
use super::models::{BreakingChange, ChangeEntry, ChangeMetrics, ChangelogResponse, ChangelogType};
use super::prompt;
use crate::common::DetailLevel;
use crate::config::Config;
use crate::debug;
use crate::git::GitRepo;
use anyhow::{Context, Result};
use chrono;
use colored::Colorize;
use regex;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

/// Struct responsible for generating changelogs
pub struct ChangelogGenerator;

impl ChangelogGenerator {
    /// Generates a changelog for the specified range of commits.
    ///
    /// # Arguments
    ///
    /// * `git_repo` - `GitRepo` instance
    /// * `from` - Starting point for the changelog (e.g., a commit hash or tag)
    /// * `to` - Ending point for the changelog (e.g., a commit hash, tag, or "HEAD")
    /// * `config` - Configuration object containing LLM settings
    /// * `detail_level` - Level of detail for the changelog (Minimal, Standard, or Detailed)
    ///
    /// # Returns
    ///
    /// A Result containing the generated changelog as a String, or an error
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
            prompt::create_changelog_system_prompt,
            prompt::create_changelog_user_prompt,
        )
        .await?;

        Ok(format_changelog_response(&changelog))
    }

    /// Updates a changelog file with new content
    ///
    /// This function reads the existing changelog file (if it exists), preserves the header,
    /// and prepends the new changelog content while maintaining the file structure.
    ///
    /// # Arguments
    ///
    /// * `changelog_content` - The new changelog content to prepend
    /// * `changelog_path` - Path to the changelog file
    /// * `git_repo` - `GitRepo` instance to use for retrieving commit dates
    /// * `to_ref` - The "to" Git reference (commit/tag) to extract the date from
    /// * `version_name` - Optional custom version name to use instead of version from Git
    ///
    /// # Returns
    ///
    /// A Result indicating success or an error
    #[allow(clippy::too_many_lines)]
    pub fn update_changelog_file(
        changelog_content: &str,
        changelog_path: &str,
        git_repo: &Arc<GitRepo>,
        to_ref: &str,
        version_name: Option<String>,
    ) -> Result<()> {
        let path = Path::new(changelog_path);
        let default_header = "# Changelog\n\nAll notable changes to this project will be documented in this file.\n\nThe format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),\nand this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n";

        // Get the date from the "to" Git reference
        let commit_date = match git_repo.get_commit_date(to_ref) {
            Ok(date) => {
                debug!("Got commit date for {}: {}", to_ref, date);
                date
            }
            Err(e) => {
                debug!("Failed to get commit date for {}: {}", to_ref, e);
                chrono::Local::now().format("%Y-%m-%d").to_string()
            }
        };

        // Strip ANSI color codes
        let stripped_content = strip_ansi_codes(changelog_content);

        // Skip the separator line if it exists (the first line with "━━━" or similar)
        let clean_content =
            if stripped_content.starts_with("━") || stripped_content.starts_with('-') {
                // Find the first newline and skip everything before it
                if let Some(pos) = stripped_content.find('\n') {
                    stripped_content[pos + 1..].to_string()
                } else {
                    stripped_content
                }
            } else {
                stripped_content
            };

        // Extract just the version content (skip the header)
        let mut version_content = if clean_content.contains("## [") {
            let parts: Vec<&str> = clean_content.split("## [").collect();
            if parts.len() > 1 {
                format!("## [{}", parts[1])
            } else {
                clean_content
            }
        } else {
            clean_content
        };

        // If version_name is provided, override the existing version
        if let Some(version) = version_name {
            if version_content.contains("## [") {
                let re = regex::Regex::new(r"## \[([^\]]+)\]").expect("Failed to compile regex");
                version_content = re
                    .replace(&version_content, &format!("## [{version}]"))
                    .to_string();
                debug!("Replaced version with user-provided version: {}", version);
            } else {
                debug!("Could not find version header to replace in changelog content");
            }
        }

        // Ensure version content has a date
        if version_content.contains(" - \n") {
            // Replace empty date placeholder with the commit date
            version_content = version_content.replace(" - \n", &format!(" - {commit_date}\n"));
            debug!("Replaced empty date with commit date: {}", commit_date);
        } else if version_content.contains("] - ") && !version_content.contains("] - 20") {
            // For cases where there's no date but a dash
            let parts: Vec<&str> = version_content.splitn(2, "] - ").collect();
            if parts.len() == 2 {
                version_content = format!(
                    "{}] - {}\n{}",
                    parts[0],
                    commit_date,
                    parts[1].trim_start_matches(['\n', ' '])
                );
                debug!("Added commit date after dash: {}", commit_date);
            }
        } else if !version_content.contains("] - ") {
            // If no date pattern at all, find the version line and add a date
            let line_end = version_content.find('\n').unwrap_or(version_content.len());
            let version_line = &version_content[..line_end];

            if version_line.contains("## [") && version_line.contains(']') {
                // Insert the date right after the closing bracket
                let bracket_pos = version_line
                    .rfind(']')
                    .expect("Failed to find closing bracket in version line");
                version_content = format!(
                    "{} - {}{}",
                    &version_content[..=bracket_pos],
                    commit_date,
                    &version_content[bracket_pos + 1..]
                );
                debug!("Added date to version line: {}", commit_date);
            }
        }

        // Add a decorative separator after the version content
        let separator =
            "\n<!-- -------------------------------------------------------------- -->\n\n";
        let version_content_with_separator = format!("{version_content}{separator}");

        let updated_content = if path.exists() {
            let existing_content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read changelog file: {changelog_path}"))?;

            // Check if the file already has a Keep a Changelog header
            if existing_content.contains("# Changelog")
                && existing_content.contains("Keep a Changelog")
            {
                // Split at the first version heading
                if existing_content.contains("## [") {
                    let parts: Vec<&str> = existing_content.split("## [").collect();
                    let header = parts[0];

                    // Combine header with new version content and existing versions
                    if parts.len() > 1 {
                        let existing_versions = parts[1..].join("## [");
                        format!("{header}{version_content_with_separator}## [{existing_versions}")
                    } else {
                        format!("{header}{version_content_with_separator}")
                    }
                } else {
                    // No version sections yet, just append new content
                    format!("{existing_content}{version_content_with_separator}")
                }
            } else {
                // Existing file doesn't have proper format, overwrite with default structure
                format!("{default_header}{version_content_with_separator}")
            }
        } else {
            // File doesn't exist, create new with proper header
            format!("{default_header}{version_content_with_separator}")
        };

        // Write the updated content back to the file
        let mut file = fs::File::create(path)
            .with_context(|| format!("Failed to create changelog file: {changelog_path}"))?;

        file.write_all(updated_content.as_bytes())
            .with_context(|| format!("Failed to write to changelog file: {changelog_path}"))?;

        Ok(())
    }
}

/// Strips ANSI color/style codes from a string
fn strip_ansi_codes(s: &str) -> String {
    // This regex matches ANSI escape codes like colors and styles
    let re = regex::Regex::new(r"\x1B\[([0-9]{1,2}(;[0-9]{1,2})*)?[m|K]")
        .expect("Failed to compile ANSI escape code regex");
    re.replace_all(s, "").to_string()
}

/// Formats the `ChangelogResponse` into a human-readable changelog
fn format_changelog_response(response: &ChangelogResponse) -> String {
    let mut formatted = String::new();

    // Add header
    formatted.push_str(&"# Changelog\n\n".bright_cyan().bold().to_string());
    formatted.push_str("All notable changes to this project will be documented in this file.\n\n");
    formatted.push_str(
        "The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),\n",
    );
    formatted.push_str("and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n");

    // Add version and release date - don't provide a date here, it will be set later
    let version = response
        .version
        .clone()
        .unwrap_or_else(|| "Unreleased".to_string());

    write!(formatted, "## [{}] - \n\n", version.bright_green().bold())
        .expect("writing to string should never fail");

    // Define the order of change types
    let ordered_types = [
        ChangelogType::Added,
        ChangelogType::Changed,
        ChangelogType::Fixed,
        ChangelogType::Removed,
        ChangelogType::Deprecated,
        ChangelogType::Security,
    ];

    // Add changes in the specified order
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

    // Add breaking changes
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

    // Add metrics
    formatted.push_str(&"### 📊 Metrics\n\n".bright_magenta().bold().to_string());
    formatted.push_str(&format_metrics(&response.metrics));

    formatted
}

/// Formats a change type with an appropriate emoji
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

/// Formats a single change entry
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

/// Formats a breaking change
fn format_breaking_change(breaking_change: &BreakingChange) -> String {
    format!(
        "- {} ({})\n",
        breaking_change.description,
        breaking_change.commit_hash.dimmed()
    )
}

/// Formats the change metrics
fn format_metrics(metrics: &ChangeMetrics) -> String {
    format!(
        "- Total Commits: {}\n- Files Changed: {}\n- Insertions: {}\n- Deletions: {}\n",
        metrics.total_commits.to_string().green(),
        metrics.files_changed.to_string().yellow(),
        metrics.insertions.to_string().green(),
        metrics.deletions.to_string().red()
    )
}
