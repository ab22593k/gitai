use super::relevance::RelevanceScorer;
use super::review::{GeneratedReview, QualityDimension};
use super::types::GeneratedMessage;
use crate::common::get_combined_instructions;
use crate::config::Config;
use crate::core::context::{ChangeType, CommitContext, ProjectMetadata, RecentCommit, StagedFile};
use crate::core::template::{
    CommitSystemTemplateContext, CommitUserTemplateContext, render_template,
};

use log::debug;
use std::collections::HashMap;
use std::fmt::Write;

pub fn create_system_prompt(config: &Config) -> anyhow::Result<String> {
    let commit_schema = schemars::schema_for!(GeneratedMessage);
    let commit_schema_str = serde_json::to_string_pretty(&commit_schema)?;

    let context = CommitSystemTemplateContext {
        config,
        schema: serde_json::from_str(&commit_schema_str)?,
        combined_instructions: get_combined_instructions(config),
    };

    render_template("commit_system", &context)
}

pub fn create_user_prompt(context: &CommitContext) -> String {
    let scorer = RelevanceScorer::new();
    let relevance_scores = scorer.score(context);
    let detailed_changes = format_detailed_changes(&context.staged_files, &relevance_scores);

    let template_context = CommitUserTemplateContext {
        context,
        recent_commits: format_recent_commits(&context.recent_commits),
        staged_changes: format_staged_files(&context.staged_files, &relevance_scores),
        project_metadata: format_project_metadata(&context.project_metadata),
        detailed_changes,
    };

    let result = render_template("commit_user", &template_context);

    debug!(
        "Generated commit prompt for {} files ({} added, {} modified, {} deleted)",
        context.staged_files.len(),
        context
            .staged_files
            .iter()
            .filter(|f| matches!(f.change_type, ChangeType::Added))
            .count(),
        context
            .staged_files
            .iter()
            .filter(|f| matches!(f.change_type, ChangeType::Modified))
            .count(),
        context
            .staged_files
            .iter()
            .filter(|f| matches!(f.change_type, ChangeType::Deleted))
            .count()
    );

    result.unwrap_or_else(|_| {
        // Fallback to old implementation if template fails
        format!(
            "Based on the following context, generate a Git commit message:\n\n\
            Branch: {}\n\n\
            Recent commits:\n{}\n\n\
            Staged changes:\n{}\n\n\
            Project metadata:\n{}\n\n\
            Detailed changes:\n{}",
            context.branch,
            template_context.recent_commits,
            template_context.staged_changes,
            template_context.project_metadata,
            template_context.detailed_changes
        )
    })
}

fn format_recent_commits(commits: &[RecentCommit]) -> String {
    commits
        .iter()
        .map(|commit| format!("{} - {}", &commit.hash[..7], commit.message))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_staged_files(files: &[StagedFile], relevance_scores: &HashMap<String, f32>) -> String {
    files
        .iter()
        .map(|file| {
            let relevance = relevance_scores.get(&file.path).unwrap_or(&0.0);
            format!(
                "{} ({:.2}) - {}",
                file.path,
                relevance,
                format_change_type(&file.change_type)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_project_metadata(metadata: &ProjectMetadata) -> String {
    format!(
        "Language: {}\nFramework: {}\nDependencies: {}",
        metadata.language.as_deref().unwrap_or("None"),
        metadata.framework.as_deref().unwrap_or("None"),
        metadata.dependencies.join(", ")
    )
}

fn format_detailed_changes(
    files: &[StagedFile],
    relevance_scores: &HashMap<String, f32>,
) -> String {
    let mut all_sections = Vec::new();

    // Add a statistical summary at the top
    let added_count = files
        .iter()
        .filter(|f| matches!(f.change_type, ChangeType::Added))
        .count();
    let modified_count = files
        .iter()
        .filter(|f| matches!(f.change_type, ChangeType::Modified))
        .count();
    let deleted_count = files
        .iter()
        .filter(|f| matches!(f.change_type, ChangeType::Deleted))
        .count();

    let summary = format!(
        "CHANGE SUMMARY:\n- {} file(s) added\n- {} file(s) modified\n- {} file(s) deleted\n- {} total file(s) changed",
        added_count,
        modified_count,
        deleted_count,
        files.len()
    );
    all_sections.push(summary);

    // First section: File summaries with diffs
    let diff_section = files
        .iter()
        .map(|file| {
            let relevance = relevance_scores.get(&file.path).unwrap_or(&0.0);

            format!(
                "File: {} (Relevance: {:.2})\nChange Type: {}\nAnalysis:\n{}\n\nDiff:\n{}",
                file.path,
                relevance,
                format_change_type(&file.change_type),
                file.analysis.join("\n"),
                file.diff
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    all_sections.push(format!(
        "=== DIFFS ({} files) ===\n\n{}",
        files.len(),
        diff_section
    ));

    // Second section: Full file contents (only for added/modified files)
    let content_files: Vec<_> = files
        .iter()
        .filter(|file| file.change_type != ChangeType::Deleted && file.content.is_some())
        .collect();

    if !content_files.is_empty() {
        let content_section = content_files
            .iter()
            .map(|file| {
                let change_indicator = match file.change_type {
                    ChangeType::Added | ChangeType::Deleted => "",
                    ChangeType::Modified => "✏️",
                };

                format!(
                    "{} File: {}\nFull File Content:\n{}\n\n--- End of File ---",
                    change_indicator,
                    file.path,
                    file.content
                        .as_ref()
                        .expect("File content should be present for added/modified files")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        all_sections.push(format!(
            "=== FULL FILE CONTENTS ({} files) ===\n\n{}",
            content_files.len(),
            content_section
        ));
    }

    all_sections.join("\n\n====================\n\n")
}

fn format_change_type(change_type: &ChangeType) -> &'static str {
    match change_type {
        ChangeType::Added => "Added",
        ChangeType::Modified => "Modified",
        ChangeType::Deleted => "Deleted",
    }
}

/// Creates a system prompt for code review generation
#[allow(clippy::too_many_lines)]
pub fn create_review_system_prompt(config: &Config) -> anyhow::Result<String> {
    let review_schema = schemars::schema_for!(GeneratedReview);
    let review_schema_str = serde_json::to_string_pretty(&review_schema)?;

    let mut prompt = String::from(
        "You are an AI assistant specializing in code reviews.
        Your task is to provide a comprehensive, professional, and constructive review of the code
        changes provided. Work step-by-step and follow these guidelines exactly:

        1. Analyze the code changes carefully, focusing on:
           - Code quality and readability
           - Potential bugs or errors
           - Architecture and design patterns
           - Performance implications
           - Security considerations
           - Maintainability and testability

        2. Provide constructive feedback:
           - Be specific and actionable in your suggestions
           - Point out both strengths and areas for improvement
           - Explain why certain patterns or practices are problematic
           - Suggest alternative approaches when appropriate

        3. Focus on substantive issues:
           - Prioritize significant issues over minor stylistic concerns
           - Consider the context of the codebase and changes
           - Note potential edge cases or scenarios that might not be handled

        4. Be professional and constructive:
           - Frame feedback positively and constructively
           - Focus on the code, not the developer
           - Acknowledge good practices and improvements

        5. Analyze the following specific dimensions of code quality:
        ",
    );

    // Add each dimension's description
    for dimension in QualityDimension::all() {
        prompt.push_str(dimension.description());
    }

    prompt.push_str(
        "
        For each dimension, identify specific issues with:
        - A severity level (Critical, High, Medium, Low)
        - Line number references or specific location in the code
        - Explanation of why it's problematic
        - Concrete recommendation for improvement

        Your review should be based entirely on the information provided in the context, without any speculation or assumptions.
      ");

    prompt.push_str(get_combined_instructions(config).as_str());

    prompt.push_str(
        "
        Your response must be a valid JSON object with the following structure:

        {
          \"summary\": \"A brief summary of the changes and their quality\",
          \"code_quality\": \"An assessment of the overall code quality\",
          \"suggestions\": [\"Suggestion 1\", \"Suggestion 2\", ...],
          \"issues\": [\"Issue 1\", \"Issue 2\", ...],
          \"positive_aspects\": [\"Positive aspect 1\", \"Positive aspect 2\", ...],",
    );

    // Add each dimension to the JSON schema
    let mut is_first = true;
    for dimension in QualityDimension::all() {
        let dim_name = match dimension {
            QualityDimension::Complexity => "complexity",
            QualityDimension::Abstraction => "abstraction",
            QualityDimension::Deletion => "deletion",
            QualityDimension::Hallucination => "hallucination",
            QualityDimension::Style => "style",
            QualityDimension::Security => "security",
            QualityDimension::Performance => "performance",
            QualityDimension::Duplication => "duplication",
            QualityDimension::ErrorHandling => "error_handling",
            QualityDimension::Testing => "testing",
            QualityDimension::BestPractices => "best_practices",
        };

        if is_first {
            is_first = false;
            write!(
                &mut prompt,
                "
          \"{dim_name}\": {{
            \"issues_found\": true/false,
            \"issues\": [
              {{
                \"description\": \"Brief description\",
                \"severity\": \"Critical/High/Medium/Low\",
                \"location\": \"filename.rs:line_numbers or path/to/file.rs:lines_range\",
                \"explanation\": \"Detailed explanation of the issue\",
                \"recommendation\": \"Specific suggestion for improvement\"
              }},
              ...
            ]
          }}"
            )
            .expect("write to string should not fail");
        } else {
            write!(
                &mut prompt,
                ",
          \"{dim_name}\": {{ ... similar structure ... }}"
            )
            .expect("write to string should not fail");
        }
    }

    prompt.push_str("
        }

        Follow these steps to generate the code review:

        1. Analyze the provided context, including staged changes and project metadata.
        2. Evaluate the code quality, looking for potential issues, improvements, and good practices.
        3. Create a concise summary of the changes and their quality.
        4. Analyze each of the code quality dimensions.
        5. For each dimension with issues, list them with appropriate severity, location, explanation, and recommendation.
        6. Provide overall suggestions for improvements.
        7. Identify specific issues found across all dimensions.
        8. Acknowledge positive aspects and good practices in the code.
        9. Construct the final JSON object with all components.

        Note: It's expected that not all dimensions will have issues. For dimensions without issues, set 'issues_found' to false and provide an empty issues array.

        Here's a minimal example of the expected output format (showing only two dimensions for brevity):

        {
          \"summary\": \"The changes implement a new authentication system with good separation of concerns, but lacks proper error handling in several places.\",
          \"code_quality\": \"The code is generally well-structured with clear naming conventions. The architecture follows established patterns, but there are some inconsistencies in error handling approaches.\",
          \"suggestions\": [\"Consider implementing a consistent error handling strategy across all authentication operations\", \"Add unit tests for edge cases in the token validation logic\"],
          \"issues\": [\"Missing error handling in the user registration flow\", \"Potential race condition in token refresh mechanism\"],
          \"positive_aspects\": [\"Good separation of concerns with clear service boundaries\", \"Consistent naming conventions throughout the added components\"],
          \"complexity\": {
            \"issues_found\": true,
            \"issues\": [
              {
                \"description\": \"Complex authentication flow with excessive nesting\",
                \"severity\": \"Medium\",
                \"location\": \"src/auth/auth_service.rs:45-67\",
                \"explanation\": \"The authentication validation contains 5 levels of nesting, making it difficult to follow the logic flow.\",
                \"recommendation\": \"Extract validation steps into separate functions and use early returns to reduce nesting\"
              }
            ]
          },
          \"error_handling\": {
            \"issues_found\": true,
            \"issues\": [
              {
                \"description\": \"Missing error handling in token refresh\",
                \"severity\": \"High\",
                \"location\": \"src/auth/auth_service.rs:102-120\",
                \"explanation\": \"The token refresh function doesn't properly handle network timeouts, potentially leaving users in an inconsistent state.\",
                \"recommendation\": \"Add explicit error handling for network timeouts with appropriate user feedback\"
              }
            ]
          },
          ... (other dimensions would be included with empty issues arrays if no issues found)
        }

        Ensure that your response is a valid JSON object matching this structure.
        "
    );

    prompt.push_str(&review_schema_str);

    Ok(prompt)
}

/// Creates a user prompt for code review generation
pub fn create_review_user_prompt(context: &CommitContext) -> String {
    let scorer = RelevanceScorer::new();
    let relevance_scores = scorer.score(context);
    let detailed_changes = format_detailed_changes(&context.staged_files, &relevance_scores);

    let prompt = format!(
        "Based on the following context, generate a code review:\n\n\
        Branch: {}\n\n\
        Recent commits:\n{}\n\n\
        Staged changes:\n{}\n\n\
        Project metadata:\n{}\n\n\
        Detailed changes:\n{}",
        context.branch,
        format_recent_commits(&context.recent_commits),
        format_staged_files(&context.staged_files, &relevance_scores),
        format_project_metadata(&context.project_metadata),
        detailed_changes
    );

    debug!(
        "Generated review prompt for {} files ({} added, {} modified, {} deleted)",
        context.staged_files.len(),
        context
            .staged_files
            .iter()
            .filter(|f| matches!(f.change_type, ChangeType::Added))
            .count(),
        context
            .staged_files
            .iter()
            .filter(|f| matches!(f.change_type, ChangeType::Modified))
            .count(),
        context
            .staged_files
            .iter()
            .filter(|f| matches!(f.change_type, ChangeType::Deleted))
            .count()
    );

    prompt
}

/// Creates a system prompt for PR description generation
pub fn create_pr_system_prompt(config: &Config) -> anyhow::Result<String> {
    let pr_schema = schemars::schema_for!(super::types::GeneratedPullRequest);
    let pr_schema_str = serde_json::to_string_pretty(&pr_schema)?;

    let mut prompt = String::from(
        "You are an AI assistant specializing in generating comprehensive, professional pull request descriptions. \
        Your task is to create clear, informative, and well-structured PR descriptions based on the provided context.

        Work step-by-step and follow these guidelines exactly:

        1. Analyze the commits and changes to understand the overall purpose of the PR
        2. Create a concise, descriptive title that summarizes the main changes
        3. Write a brief summary that captures the essence of what was changed
        4. Provide a detailed description explaining what was changed, why it was changed, and how it works
        5. List all commits included in this PR for reference
        6. Identify any breaking changes and explain their impact
        7. Provide testing instructions if the changes require specific testing steps
        8. Include any additional notes or context that would be helpful for reviewers

        Guidelines for PR descriptions:
        - Focus on the overall impact and purpose, not individual commit details
        - Explain the 'why' behind changes, not just the 'what'
        - Use clear, professional language suitable for code review
        - Organize information logically with proper sections
        - Be comprehensive but concise
        - Consider the audience: other developers who need to review and understand the changes
        - Highlight any configuration changes, migrations, or deployment considerations
        - Mention any dependencies or prerequisites
        - Note any performance implications or architectural decisions

        Your description should treat the changeset as an atomic unit representing a cohesive feature,
        fix, or improvement, rather than a collection of individual commits.
        ");

    prompt.push_str(get_combined_instructions(config).as_str());

    prompt.push_str(
        "
        Your response must be a valid JSON object with the following structure:

        {
          \"emoji\": \"string or null\",
          \"title\": \"Clear, descriptive PR title\",
          \"summary\": \"Brief overview of the changes\",
          \"description\": \"Detailed explanation organized into Features section with sub-sections for Core Capabilities, Technical Details, CLI/Integration details, etc.\",
          \"commits\": [\"List of commit messages included in this PR\"],
          \"breaking_changes\": [\"Any breaking changes introduced\"],
          \"testing_notes\": \"Instructions for testing these changes (optional)\",
          \"notes\": \"Additional context or notes for reviewers (optional)\"
        }

        Follow these steps to generate the PR description:

        1. Analyze the provided context, including commit messages, file changes, and project metadata
        2. Identify the main theme or purpose that unifies all the changes
        3. Create a clear, descriptive title that captures the essence of the PR
        4. If using emojis, select the most appropriate one for the PR type
        5. Write a concise summary highlighting the key changes and their impact
        6. Organize the description into a Features section with logical sub-sections
        7. List all commit messages for reference and traceability
        8. Identify any breaking changes and explain their impact on users or systems
        9. Provide testing instructions if the changes require specific testing procedures
        10. Add any additional notes about deployment, configuration, or other considerations
        11. Construct the final JSON object with all components

        Example output format:

        {
          \"emoji\": \"✨\",
          \"title\": \"Add comprehensive Experience Fragment management system\",
          \"summary\": \"Implements full lifecycle support for Experience Fragments (XFs), including create, retrieve, update, and page integration operations. Adds a unified agent tool, rich CLI interface, and tight AEM manager integration with tenant-specific configuration support.\",
          \"description\": \"### Core Capabilities\\n\\n* Unified `manage_experience_fragments` tool with four key operations:\\n  * `create`: Create new XFs with optional initial content\\n  * `get`: Retrieve existing XF data\\n  * `update`: Modify XF content\\n  * `add_to_page`: Inject XF references into pages with flexible positioning\\n\\n* AEM manager integration with `createExperienceFragment` and `populateExperienceFragment`\\n* Support for tenant-specific `experienceFragmentComponentType` configuration\\n\\n###  Technical Details\\n\\n* Secure CSRF token handling for all operations\\n* XF page structure conversion for accurate population\\n* AEM 6.5 vs AEM Cloud component type detection\\n* Unique XF name generation with randomized suffixes\\n* Comprehensive validation and error handling\\n* State change logging for operational observability\\n\\n### 🖥 CLI Tooling\\n\\n* New command-line script with full XF management\\n* Commands: `create`, `update`, `list`, `get`, `delete`, `search`, `info`\\n* Content file input/output support\\n* XF discovery and metadata analysis tools\",
          \"commits\": [\"b1b1f3f: feat(xf): add experience fragment management system\"],
          \"breaking_changes\": [],
          \"testing_notes\": \"Verified XF creation, update, and population. Confirmed CLI command behavior across all operations. Tested page integration and position logic. Checked tenant-specific component resolution.\",
          \"notes\": \"Tenants using non-default XF components must define `experienceFragmentComponentType`. Requires sufficient AEM permissions and CSRF support.\"
        }

        Ensure that your response is a valid JSON object matching this structure. Include an empty string for the emoji if not using one.
        ");

    prompt.push_str(&pr_schema_str);

    Ok(prompt)
}

/// Creates a user prompt for PR description generation
pub fn create_pr_user_prompt(context: &CommitContext, commit_messages: &[String]) -> String {
    let scorer = RelevanceScorer::new();
    let relevance_scores = scorer.score(context);
    let detailed_changes = format_detailed_changes(&context.staged_files, &relevance_scores);

    let commits_section = if commit_messages.is_empty() {
        "No commits available".to_string()
    } else {
        commit_messages.join("\n")
    };

    let prompt = format!(
        "Based on the following context, generate a comprehensive pull request description:\n\n\
        Range: {}\n\n\
        Commits in this PR:\n{}\n\n\
        Recent commit history:\n{}\n\n\
        File changes summary:\n{}\n\n\
        Project metadata:\n{}\n\n\
        Detailed changes:\n{}",
        context.branch,
        commits_section,
        format_recent_commits(&context.recent_commits),
        format_staged_files(&context.staged_files, &relevance_scores),
        format_project_metadata(&context.project_metadata),
        detailed_changes
    );

    debug!(
        "Generated PR prompt for {} files across {} commits",
        context.staged_files.len(),
        commit_messages.len()
    );

    prompt
}
