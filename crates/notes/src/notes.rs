use crate::models::{Highlight, ReleaseNotesResponse, Section, SectionItem};
use anyhow::Result;
use cloy::commands::changelog::change_analyzer::AnalyzedChange;
use cloy::commands::changelog::common::generate_changes_content;
use cloy::commands::changelog::models::{BreakingChange, ChangeMetrics};
use cloy::common::DetailLevel;
use cloy::config::Config;
use cloy::git::GitRepo;
use colored::Colorize;
use prompts::notes as notes_prompts;
use std::fmt::Write as FmtWrite;
use std::sync::Arc;

/// Struct responsible for generating release notes
pub struct ReleaseNotesGenerator;

impl ReleaseNotesGenerator {
    /// Generates release notes for the specified range of commits.
    pub async fn generate(
        git_repo: Arc<GitRepo>,
        from: &str,
        to: &str,
        config: &Config,
        detail_level: DetailLevel,
        version_name: Option<String>,
    ) -> Result<String> {
        let release_notes: ReleaseNotesResponse = generate_changes_content::<ReleaseNotesResponse>(
            git_repo,
            from,
            to,
            config,
            detail_level,
            system_prompt_adapter,
            user_prompt_adapter,
        )
        .await?;

        Ok(format_release_notes_response(
            &release_notes,
            version_name.as_deref(),
        ))
    }
}

fn system_prompt_adapter(config: &Config) -> String {
    let schema = schemars::schema_for!(ReleaseNotesResponse);
    let schema_str = serde_json::to_string_pretty(&schema).unwrap_or_else(|_| String::from("{}"));
    let instructions = cloy::common::get_combined_instructions(config);
    notes_prompts::create_release_notes_system_prompt(&instructions, &schema_str)
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
            "EXIGENCY: Brief technical summary focusing on critical capabilities."
        }
        DetailLevel::Standard => {
            "EXIGENCY: Balanced overview of new technical features and architectural improvements."
        }
        DetailLevel::Detailed => {
            "EXIGENCY: Comprehensive technical narrative including deep context and rationale."
        }
    };

    notes_prompts::create_release_notes_user_prompt(
        from,
        to,
        &metrics_buf,
        &changes_buf,
        readme_summary,
        detail_req,
    )
}

/// Formats the `ReleaseNotesResponse` into human-readable release notes
fn format_release_notes_response(
    response: &ReleaseNotesResponse,
    version_name: Option<&str>,
) -> String {
    let mut formatted = String::new();

    let version = match version_name {
        Some(name) => name.to_string(),
        None => response
            .version
            .clone()
            .unwrap_or_else(|| "Unreleased".to_string()),
    };

    write!(
        formatted,
        "# Release Notes - v{}\n\n",
        version.bright_green().bold()
    )
    .expect("writing to string should never fail");
    write!(
        formatted,
        "Release Date: {}\n\n",
        response
            .release_date
            .clone()
            .unwrap_or_else(|| "N/A".to_string())
            .yellow()
    )
    .expect("writing to string should never fail");

    write!(formatted, "{}\n\n", response.summary.bright_cyan())
        .expect("writing to string should never fail");

    if !response.highlights.is_empty() {
        formatted.push_str(&"## ✨ Highlights\n\n".bright_magenta().bold().to_string());
        for highlight in &response.highlights {
            formatted.push_str(&format_highlight(highlight));
        }
    }

    for section in &response.sections {
        formatted.push_str(&format_section(section));
    }

    if !response.breaking_changes.is_empty() {
        formatted.push_str(&"## ⚠️ Breaking Changes\n\n".bright_red().bold().to_string());
        for breaking_change in &response.breaking_changes {
            formatted.push_str(&format_breaking_change(breaking_change));
        }
    }

    if !response.upgrade_notes.is_empty() {
        formatted.push_str(&"## 🔧 Upgrade Notes\n\n".yellow().bold().to_string());
        for note in &response.upgrade_notes {
            writeln!(formatted, "- {note}").expect("writing to string should never fail");
        }
        formatted.push('\n');
    }

    formatted.push_str(&"## 📊 Metrics\n\n".bright_blue().bold().to_string());
    formatted.push_str(&format_metrics(&response.metrics));

    formatted
}

fn format_highlight(highlight: &Highlight) -> String {
    format!(
        "### {}\n\n{}\n\n",
        highlight.title.bright_yellow().bold(),
        highlight.description
    )
}

fn format_section(section: &Section) -> String {
    let mut formatted = format!("## {}\n\n", section.title.bright_blue().bold());
    for item in &section.items {
        formatted.push_str(&format_section_item(item));
    }
    formatted.push('\n');
    formatted
}

fn format_section_item(item: &SectionItem) -> String {
    let mut formatted = format!("- {}", item.description);

    if !item.associated_issues.is_empty() {
        write!(
            formatted,
            " ({})",
            item.associated_issues.join(", ").yellow()
        )
        .expect("writing to string should never fail");
    }

    if let Some(pr) = &item.pull_request {
        write!(formatted, " [{}]", pr.bright_purple())
            .expect("writing to string should never fail");
    }

    formatted.push('\n');
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
