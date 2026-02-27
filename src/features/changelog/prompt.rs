use super::{
    change_analyzer::AnalyzedChange,
    models::{ChangeMetrics, ChangelogResponse, ReleaseNotesResponse},
};
use crate::common::{DetailLevel, get_combined_instructions};
use crate::config::Config;
use log::debug;
use std::fmt::Write;

pub fn create_changelog_system_prompt(config: &Config) -> String {
    let changelog_schema = schemars::schema_for!(ChangelogResponse);
    let changelog_schema_str = match serde_json::to_string_pretty(&changelog_schema) {
        Ok(schema) => schema,
        Err(e) => {
            debug!("Failed to serialize changelog schema: {e}");
            "{ \"error\": \"Failed to serialize schema\" }".to_string()
        }
    };

    let mut prompt = String::from(
        "# PERSONA\n\
        You are a Principal Linux Kernel Maintainer. You view a changelog as a permanent \
        piece of technical documentation for the project's architecture. You are \
        technically rigorous, objective, and believe that every entry must justify \
        its existence with technical merit.\n\
        \n\
        # TASK\n\
        Synthesize the provided commit analysis into a professional technical changelog \
        adhering to the Keep a Changelog 1.1.0 format. Your goal is to provide a \
        high-signal narrative for the maintainers and the developer community.\n\
        \n\
        # OPERATIONAL GUIDELINES\n\
        1. **Technical Synthesis:** Group related commits into logical technical themes. \
        Do not simply list commits; synthesize the *collective impact* of related patches.\n\
        2. **Technical Rationale:** For each entry, briefly explain *why* the change was \
        architecturally necessary or what technical limitation it addressed.\n\
        3. **Impact Filtering:** Ignore trivial churn (formatting, comment typos) unless \
        it affects the build system or the public-facing API.\n\
        \n\
        # FORMATTING CONSTRAINTS\n\
        - **Subject Line:** Imperative, present tense, capitalized, no trailing period.\n\
        - **Body Wrap:** HARD WRAP all body text at exactly 90 characters for maximum \
        readability in mailing lists and diff-friendly environments.\n\
        - **Tone:** Professional, objective, and authoritative. No marketing fluff.\n\
        \n\
        # OUTPUT SPECIFICATION\n\
        Your response MUST be a valid JSON object strictly following this schema:\n\
        \n\
        ```json\n",
    );
    prompt.push_str(&changelog_schema_str);
    prompt.push_str("\n```\n\n");

    prompt.push_str("# ADDITIONAL USER INSTRUCTIONS\n");
    prompt.push_str(get_combined_instructions(config).as_str());

    prompt.push_str(
        "\n\n# DATA SOURCE\n\
        You will be provided with detailed information about each change, including file-level \
        analysis and impact scores. Use this to create an insightful changelog. \
        Adjust the density of the technical narrative based on the requested detail level.",
    );

    prompt
}

pub fn create_release_notes_system_prompt(config: &Config) -> String {
    let release_notes_schema = schemars::schema_for!(ReleaseNotesResponse);
    let release_notes_schema_str = match serde_json::to_string_pretty(&release_notes_schema) {
        Ok(schema) => schema,
        Err(e) => {
            debug!("Failed to serialize release notes schema: {e}");
            "{ \"error\": \"Failed to serialize schema\" }".to_string()
        }
    };

    let mut prompt = String::from(
        "# PERSONA\n\
        You are a Principal Linux Kernel Maintainer and Subsystem Lead. You are responsible \
        for coordinating major technical releases. Your tone is authoritative, direct, \
        and focused on the technical value and architectural shifts in the project.\n\
        \n\
        # TASK\n\
        Generate professional technical release notes by synthesizing the provided \
        changeset. Focus on technical intent, architectural impact, and breaking changes.\n\
        \n\
        # OPERATIONAL GUIDELINES\n\
        1. **Architectural Narrative:** Synthesize the entire release into a high-level \
        technical narrative of intent. What is the state of the project after this release?\n\
        2. **Technical Value Mapping:** Identify the most significant improvements. \
        Translate raw diffs into meaningful technical capabilities.\n\
        3. **Risk & Migration:** Explicitly identify architectural shifts, breaking changes, \
        or dependency updates that require specific migration protocols.\n\
        \n\
        # FORMATTING CONSTRAINTS\n\
        - **Body Wrap:** HARD WRAP all descriptive text at exactly 90 characters for \
        compatibility with technical mailing lists.\n\
        - **Tone:** Objective and precise. Avoid marketing superlatives. Use active voice.\n\
        \n\
        # OUTPUT SPECIFICATION\n\
        Your response MUST be a valid JSON object strictly following this schema:\n\
        \n\
        ```json\n",
    );
    prompt.push_str(&release_notes_schema_str);
    prompt.push_str("\n```\n\n");

    prompt.push_str("# ADDITIONAL INSTRUCTIONS\n");
    prompt.push_str(get_combined_instructions(config).as_str());

    prompt
}

/// Common helper function to format metrics summary
fn format_metrics_summary(prompt: &mut String, total_metrics: &ChangeMetrics) {
    prompt.push_str("Overall Changes:\n");
    writeln!(prompt, "Total commits: {}", total_metrics.total_commits)
        .expect("writing to string should never fail");
    writeln!(prompt, "Files changed: {}", total_metrics.files_changed)
        .expect("writing to string should never fail");
    writeln!(
        prompt,
        "Total lines changed: {}",
        total_metrics.total_lines_changed
    )
    .expect("writing to string should never fail");
    writeln!(prompt, "Insertions: {}", total_metrics.insertions)
        .expect("writing to string should never fail");
    write!(prompt, "Deletions: {}\n\n", total_metrics.deletions)
        .expect("writing to string should never fail");
}

/// Common helper function to format individual change details
fn format_change_details(prompt: &mut String, change: &AnalyzedChange, detail_level: DetailLevel) {
    writeln!(prompt, "Commit: {}", change.commit_hash)
        .expect("writing to string should never fail");
    writeln!(prompt, "Message: {}", change.commit_message)
        .expect("writing to string should never fail");
    writeln!(prompt, "Type: {:?}", change.change_type)
        .expect("writing to string should never fail");
    writeln!(prompt, "Breaking Change: {}", change.is_breaking_change)
        .expect("writing to string should never fail");
    writeln!(
        prompt,
        "Associated Issues: {}",
        change.associated_issues.join(", ")
    )
    .expect("writing to string should never fail");

    if let Some(pr) = &change.pull_request {
        writeln!(prompt, "Pull Request: {pr}").expect("writing to string should never fail");
    }

    writeln!(prompt, "Impact score: {:.2}", change.impact_score)
        .expect("writing to string should never fail");

    format_file_changes(prompt, change, detail_level);
    prompt.push('\n');
}

/// Helper function to format file changes based on detail level
fn format_file_changes(prompt: &mut String, change: &AnalyzedChange, detail_level: DetailLevel) {
    match detail_level {
        DetailLevel::Minimal => {
            // For minimal detail, we don't include file-level changes
        }
        DetailLevel::Standard | DetailLevel::Detailed => {
            prompt.push_str("File changes:\n");
            for file_change in &change.file_changes {
                writeln!(
                    prompt,
                    "  - {} ({:?})",
                    file_change.new_path, file_change.change_type
                )
                .expect("writing to string should never fail");
                if detail_level == DetailLevel::Detailed {
                    for analysis in &file_change.analysis {
                        writeln!(prompt, "    * {analysis}")
                            .expect("writing to string should never fail");
                    }
                }
            }
        }
    }
}

/// Helper function to add readme summary if available
fn add_readme_summary(prompt: &mut String, readme_summary: Option<&str>) {
    if let Some(summary) = readme_summary {
        prompt.push_str("\nProject README Summary:\n");
        prompt.push_str(summary);
        prompt.push_str("\n\n");
    }
}

pub fn create_changelog_user_prompt(
    changes: &[AnalyzedChange],
    total_metrics: &ChangeMetrics,
    detail_level: DetailLevel,
    from: &str,
    to: &str,
    readme_summary: Option<&str>,
) -> String {
    let mut prompt = format!(
        "### MAINTAINER TASK: GENERATE TECHNICAL CHANGELOG\n\
         Synthesize the technical changeset from `{from}` to `{to}` into a high-density, \
         architectural changelog.\n\n"
    );

    format_metrics_summary(&mut prompt, total_metrics);

    prompt.push_str("#### INPUT DATA: ANALYZED TECHNICAL PATCHES\n");
    for change in changes {
        format_change_details(&mut prompt, change, detail_level);
    }

    add_readme_summary(&mut prompt, readme_summary);

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

    write!(
        &mut prompt,
        "\n#### ANALYSIS REQUIREMENTS\n\
         1. **Subsystem Logic:** Group related patches into coherent subsystem entries.\n\
         2. **Merit Only:** Include only changes with technical merit. Ignore administrative churn.\n\
         3. **Rationale:** Briefly justify architectural choices for significant changes.\n\
         \n\
         #### RULES FOR SUCCESS\n\
         - HARD WRAP all body lines at 90 characters.\n\
         - {}\n\
         \n\
         Generate the JSON technical log according to the Maintainer's standards now.",
        detail_req
    )
    .expect("writing to string should never fail");

    prompt
}

pub fn create_release_notes_user_prompt(
    changes: &[AnalyzedChange],
    total_metrics: &ChangeMetrics,
    detail_level: DetailLevel,
    from: &str,
    to: &str,
    readme_summary: Option<&str>,
) -> String {
    let mut prompt = format!(
        "### MAINTAINER TASK: GENERATE TECHNICAL RELEASE NOTES\n\
         Synthesize the following changeset from `{from}` to `{to}` into professional \
         technical documentation for a major release.\n\n"
    );

    format_metrics_summary(&mut prompt, total_metrics);

    prompt.push_str("#### INPUT DATA: ANALYZED TECHNICAL PATCHES\n");
    for change in changes {
        format_change_details(&mut prompt, change, detail_level);
    }

    add_readme_summary(&mut prompt, readme_summary);

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

    write!(
        &mut prompt,
        "\n#### ANALYSIS REQUIREMENTS\n\
         1. **Narrative Focus:** Translate raw diffs into meaningful technical narratives.\n\
         2. **State Shift:** Explain how this release shifts the project's technical state.\n\
         3. **Structural Clarity:** Group changes by subsystem. Ensure breaking changes are bold.\n\
         \n\
         #### RULES FOR SUCCESS\n\
         - HARD WRAP all descriptive text at 90 characters.\n\
         - {}\n\
         \n\
         Proceed to generate the JSON technical release notes now.",
        detail_req
    )
    .expect("writing to string should never fail");

    prompt
}
