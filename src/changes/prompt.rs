use super::{
    change_analyzer::AnalyzedChange,
    models::{ChangeMetrics, ChangelogResponse, ReleaseNotesResponse},
};
use crate::common::{DetailLevel, get_combined_instructions};
use crate::config::Config;
use crate::debug;
use std::fmt::Write;

pub fn create_changelog_system_prompt(config: &Config) -> String {
    let changelog_schema = schemars::schema_for!(ChangelogResponse);
    let changelog_schema_str = match serde_json::to_string_pretty(&changelog_schema) {
        Ok(schema) => schema,
        Err(e) => {
            debug!("Failed to serialize changelog schema: {}", e);
            "{ \"error\": \"Failed to serialize schema\" }".to_string()
        }
    };

    let mut prompt = String::from(
        "You are an AI assistant specialized in generating clear, concise, and informative changelogs for software projects. \
        Your task is to create a well-structured changelog based on the provided commit information and analysis. \
        The changelog should adhere to the Keep a Changelog 1.1.0 format (https://keepachangelog.com/en/1.1.0/).

        Work step-by-step and follow these guidelines exactly:

        1. Categorize changes into the following types: Added, Changed, Deprecated, Removed, Fixed, Security.
        2. Use present tense and imperative mood in change descriptions.
        3. Start each change entry with a capital letter and do not end with a period.
        4. Be concise but descriptive in change entries and ensure good grammar, capitalization, and punctuation.
        5. Include *short* commit hashes at the end of each entry.
        6. Focus on the impact and significance of the changes and omit trivial changes below the relevance threshold.
        7. Find commonalities and group related changes together under the appropriate category.
        8. List the most impactful changes first within each category.
        9. Mention associated issue numbers and pull request numbers when available.
        10. Clearly identify and explain any breaking changes.
        11. Avoid common cliché words (like 'enhance', 'streamline', 'leverage', etc) and phrases.
        12. Do not speculate about the purpose of a change or add any information not directly supported by the context.
        13. Mention any changes to dependencies or build configurations under the appropriate category.
        14. Highlight changes that affect multiple parts of the codebase or have cross-cutting concerns.
        15. NO YAPPING!

        Your response must be a valid JSON object with the following structure:

        {
          \"version\": \"string or null\",
          \"release_date\": \"string or null\",
          \"sections\": {
            \"Added\": [{ \"description\": \"string\", \"commit_hashes\": [\"string\"], \"associated_issues\": [\"string\"], \"pull_request\": \"string or null\" }],
            \"Changed\": [...],
            \"Deprecated\": [...],
            \"Removed\": [...],
            \"Fixed\": [...],
            \"Security\": [...]
          },
          \"breaking_changes\": [{ \"description\": \"string\", \"commit_hash\": \"string\" }],
          \"metrics\": {
            \"total_commits\": number,
            \"files_changed\": number,
            \"insertions\": number,
            \"deletions\": number
          }
        }

        Follow these steps to generate the changelog:

        1. Analyze the provided commit information and group changes by type (Added, Changed, etc.).
        2. For each change type, create an array of change entries with description, commit hashes, associated issues, and pull request (if available).
        3. Identify any breaking changes and add them to the breaking_changes array.
        4. Calculate the metrics based on the overall changes.
        5. If provided, include the version and release date.
        6. Construct the final JSON object ensuring all required fields are present.

        Here's a minimal example of the expected output format:

        {
          \"version\": \"1.0.0\",
          \"release_date\": \"2023-08-15\",
          \"sections\": {
            \"Added\": [
              {
                \"description\": \"add new feature X\",
                \"commit_hashes\": [\"abc123\"],
              }
            ],
            \"Changed\": [],
            \"Deprecated\": [],
            \"Removed\": [],
            \"Fixed\": [],
            \"Security\": []
          },
          \"breaking_changes\": [],
          \"metrics\": {
            \"total_commits\": 1,
            \"files_changed\": 3,
            \"insertions\": 100,
            \"deletions\": 50
          }
        }

        Ensure that your response is a valid JSON object matching this structure. Include all required fields, even if they are empty arrays or null values.
        "
    );

    prompt.push_str(&changelog_schema_str);

    prompt.push_str(get_combined_instructions(config).as_str());

    prompt.push_str(
        "\n\nYou will be provided with detailed information about each change, including file-level analysis, impact scores, and classifications. \
        Use this information to create a comprehensive and insightful changelog. \
        Adjust the level of detail based on the specified detail level (Minimal, Standard, or Detailed)."
    );

    prompt
}

pub fn create_release_notes_system_prompt(config: &Config) -> String {
    let release_notes_schema = schemars::schema_for!(ReleaseNotesResponse);
    let release_notes_schema_str = match serde_json::to_string_pretty(&release_notes_schema) {
        Ok(schema) => schema,
        Err(e) => {
            debug!("Failed to serialize release notes schema: {}", e);
            "{ \"error\": \"Failed to serialize schema\" }".to_string()
        }
    };

    let mut prompt = String::from(
        "You are an AI assistant specialized in generating comprehensive and user-friendly release notes for software projects. \
        Your task is to create detailed release notes based on the provided commit information and analysis. \
        Aim for a tone that is professional, approachable, and authoritative, keeping in mind any additional user instructions.

        Work step-by-step and follow these guidelines exactly:

        1. Provide a high-level summary of the release, highlighting key features, improvements, and fixes.
        2. Find commonalities and group changes into meaningful sections (e.g., 'New Features', 'Improvements', 'Bug Fixes', 'Breaking Changes').
        3. Focus on the impact and benefits of the changes to users and developers.
        4. Highlight any significant new features or major improvements.
        5. Explain the rationale behind important changes when possible.
        6. Note any breaking changes and provide clear upgrade instructions.
        7. Mention any changes to dependencies or system requirements.
        8. Include any relevant documentation updates or new resources for users.
        9. Use clear, non-technical language where possible to make the notes accessible to a wide audience.
        10. Provide context for technical changes when necessary.
        11. Highlight any security updates or important bug fixes.
        12. Include overall metrics to give context about the scope of the release.
        13. Mention associated issue numbers and pull request numbers when relevant.
        14. NO YAPPING!

        Your response must be a valid JSON object with the following structure:

        {
          \"version\": \"string or null\",
          \"release_date\": \"string or null\",
          \"sections\": {
            \"Added\": [{ \"description\": \"string\", \"commit_hashes\": [\"string\"], \"associated_issues\": [\"string\"], \"pull_request\": \"string or null\" }],
            \"Changed\": [...],
            \"Deprecated\": [...],
            \"Removed\": [...],
            \"Fixed\": [...],
            \"Security\": [...]
          },
          \"breaking_changes\": [{ \"description\": \"string\", \"commit_hash\": \"string\" }],
          \"metrics\": {
            \"total_commits\": number,
            \"files_changed\": number,
            \"insertions\": number,
            \"deletions\": number
            \"total_lines_changed\": number
          }
        }

        Follow these steps to generate the changelog:

        1. Analyze the provided commit information and group changes by type (Added, Changed, etc.).
        2. For each change type, create an array of change entries with description, commit hashes, associated issues, and pull request (if available).
        3. Identify any breaking changes and add them to the breaking_changes array.
        4. Calculate the metrics based on the overall changes.
        5. If provided, include the version and release date.
        6. Construct the final JSON object ensuring all required fields are present.

        Here's a minimal example of the expected output format:

        {
          \"version\": \"1.0.0\",
          \"release_date\": \"2023-08-15\",
          \"sections\": {
            \"Added\": [
              {
                \"description\": \"add new feature X\",
                \"commit_hashes\": [\"abc123\"],
                \"associated_issues\": [\"#42\"],
                \"pull_request\": \"PR #100\"
              }
            ],
            \"Changed\": [],
            \"Deprecated\": [],
            \"Removed\": [],
            \"Fixed\": [],
            \"Security\": []
          },
          \"breaking_changes\": [],
          \"metrics\": {
            \"total_commits\": 1,
            \"files_changed\": 3,
            \"insertions\": 100,
            \"deletions\": 50
            \"total_lines_changed\": 150
          }
        }

        Ensure that your response is a valid JSON object matching this structure. Include all required fields, even if they are empty arrays or null values.
        "
    );

    prompt.push_str(&release_notes_schema_str);
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
    writeln!(prompt, "Author: {}", change.author).expect("writing to string should never fail");
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
    let mut prompt =
        format!("Based on the following changes from {from} to {to}, generate a changelog:\n\n");

    format_metrics_summary(&mut prompt, total_metrics);

    for change in changes {
        format_change_details(&mut prompt, change, detail_level);
    }

    add_readme_summary(&mut prompt, readme_summary);

    write!(&mut prompt, "Please generate a {} changelog for the changes from {} to {}, adhering to the Keep a Changelog format. ",
        match detail_level {
            DetailLevel::Minimal => "concise",
            DetailLevel::Standard => "comprehensive",
            DetailLevel::Detailed => "highly detailed",
        },
        from,
        to
    ).expect("writing to string should never fail");

    prompt.push_str("Categorize the changes appropriately and focus on the most significant updates and their impact on the project. ");
    prompt.push_str("For each change, provide a clear description of what was changed, adhering to the guidelines in the system prompt. ");
    prompt.push_str("Include the commit hashes, associated issues, and pull request numbers for each entry when available. ");
    prompt.push_str("Clearly identify and explain any breaking changes. ");

    if readme_summary.is_some() {
        prompt.push_str("Use the README summary to provide context about the project and ensure the changelog reflects the project's goals and main features. ");
    }

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
    let mut prompt =
        format!("Based on the following changes from {from} to {to}, generate release notes:\n\n");

    format_metrics_summary(&mut prompt, total_metrics);

    for change in changes {
        format_change_details(&mut prompt, change, detail_level);
    }

    add_readme_summary(&mut prompt, readme_summary);

    write!(
        &mut prompt,
        "Please generate {} release notes for the changes from {} to {}. ",
        match detail_level {
            DetailLevel::Minimal => "concise",
            DetailLevel::Standard => "comprehensive",
            DetailLevel::Detailed => "highly detailed",
        },
        from,
        to
    )
    .expect("writing to string should never fail");

    match detail_level {
        DetailLevel::Minimal => {
            prompt.push_str(
                "Keep the release notes brief and focus only on the most critical changes. ",
            );
        }
        DetailLevel::Standard => {
            prompt.push_str("Provide a balanced overview of all significant changes. ");
        }
        DetailLevel::Detailed => {
            prompt.push_str("Include detailed explanations and context for all changes. ");
        }
    }

    prompt.push_str("Focus on user-facing changes and highlight the most impactful improvements, new features, and bug fixes. ");
    prompt.push_str("Structure the notes to be clear and actionable for users and developers. ");
    prompt.push_str("Include upgrade notes for any breaking changes. ");
    prompt.push_str("Reference associated issues and pull requests where relevant. ");

    if readme_summary.is_some() {
        prompt.push_str("Use the README summary to understand the project context and ensure the release notes align with the project's purpose and user base. ");
    }

    prompt
}
