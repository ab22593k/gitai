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
        You are a Lead Maintainer and Technical Architect. You view a changelog as a permanent \
        record of a project's evolution. You are technically rigorous, prioritize technical \
        accuracy over marketing fluff, and demand clarity in every entry.\n\
        \n\
        # TASK\n\
        Synthesize the provided commit analysis into a professional changelog adhering to the \
        Keep a Changelog 1.1.0 format. Your goal is to provide a high-signal technical summary \
        for other engineers.\n\
        \n\
        # ANALYTICAL PROTOCOL (Chain of Thought)\n\
        1. **Technical Synthesis:** Group related commits into logical technical themes. Do not \
        simply list commits; synthesize the *collective impact* of related patches.\n\
        2. **Impact Filtering:** Ignore trivial changes (formatting, typo fixes in comments) unless \
        they affect the build or user-facing API.\n\
        3. **Narrative Generation:** For each category, write entries that explain the solution \
        and its rationale in the imperative mood.\n\
        \n\
        # OPERATIONAL CONSTRAINTS\n\
        - **Subject Format:** Imperative, present tense, capitalized, no trailing period.\n\
        - **No Fluff:** **Negative Constraint:** NEVER use verbs like \"enhanced\", \"optimized\", \
        or \"streamlined\" without providing a specific technical metric or rationale.\n\
        - **Technical Precision:** Identify breaking changes with absolute clarity.\n\
        - **No Yapping:** Output must be the JSON object and nothing else.\n\
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
        You are a Technical Lead responsible for coordinating major releases. Your tone is \
        authoritative, direct, and focused on the value provided to the end-user and developer community.\n\
        \n\
        # TASK\n\
        Generate professional, user-friendly release notes by synthesizing the provided changeset. \
        Focus on impact, breaking changes, and technical narratives that matter.\n\
        \n\
        # ANALYTICAL PROTOCOL\n\
        1. **Value Mapping:** Identify the most significant new features and improvements. \
        Translate technical diffs into functional benefits.\n\
        2. **Risk Assessment:** Explicitly look for architectural shifts or dependency updates \
        that require migration steps.\n\
        3. **Executive Summary:** Synthesize the entire release into a high-level summary of intent.\n\
        \n\
        # QUALITY GUIDELINES\n\
        - **Active Voice:** Use professional, approachable language.\n\
        - **Technical Depth:** Provide context for complex technical changes when necessary.\n\
        - **Constraint:** NO YAPPING. No conversational preambles.\n\
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
         Synthesize the following changes from `{from}` to `{to}` into a high-density, \
         professional changelog.\n\n"
    );

    format_metrics_summary(&mut prompt, total_metrics);

    prompt.push_str("#### INPUT DATA: DETAILED CHANGES\n");
    for change in changes {
        format_change_details(&mut prompt, change, detail_level);
    }

    add_readme_summary(&mut prompt, readme_summary);

    let detail_req = match detail_level {
        DetailLevel::Minimal => "EXIGENCY: Keep it technical and extremely concise.",
        DetailLevel::Standard => {
            "EXIGENCY: Provide a balanced overview of all significant changes."
        }
        DetailLevel::Detailed => {
            "EXIGENCY: Exhaustive technical narrative. Include context for major file changes."
        }
    };

    write!(
        &mut prompt,
        "\n#### ANALYSIS REQUIREMENTS\n\
         1. **Categorization:** Strictly use the categories defined in the system prompt.\n\
         2. **Synthesis:** Group related patches into coherent entries. Focus on the impact.\n\
         3. **Merit:** Only include changes with technical merit. Ignore churn.\n\
         4. **Sign-offs:** Include short commit hashes for traceability.\n\
         \n\
         {}\n\
         \n\
         Generate the JSON object according to the Maintainer's standards now.",
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
        "### TASK: GENERATE RELEASE NOTES\n\
         Synthesize the following release dataset from `{from}` to `{to}` into professional, \
         approachable release notes.\n\n"
    );

    format_metrics_summary(&mut prompt, total_metrics);

    prompt.push_str("#### DATASET: CHANGESET DETAILS\n");
    for change in changes {
        format_change_details(&mut prompt, change, detail_level);
    }

    add_readme_summary(&mut prompt, readme_summary);

    let detail_req = match detail_level {
        DetailLevel::Minimal => "EXIGENCY: Brief summary focusing only on critical new features.",
        DetailLevel::Standard => {
            "EXIGENCY: Balanced overview of features, fixes, and improvements."
        }
        DetailLevel::Detailed => {
            "EXIGENCY: Comprehensive release narrative including technical context and rationale."
        }
    };

    write!(
        &mut prompt,
        "\n#### REQUIREMENTS\n\
         1. **Value Filtering:** Highlight user-facing benefits and major new capabilities.\n\
         2. **Clarity:** Translate complex diffs into professional narratives. Avoid jargon unless necessary.\n\
         3. **Structure:** Group changes logically. Ensure breaking changes are impossible to miss.\n\
         \n\
         {}\n\
         \n\
         Proceed to generate the JSON release notes now.",
        detail_req
    )
    .expect("writing to string should never fail");

    prompt
}
