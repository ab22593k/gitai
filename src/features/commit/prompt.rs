use super::types::GeneratedMessage;
use crate::common::{DetailLevel, get_combined_instructions};
use crate::config::Config;
use crate::core::context::{ChangeType, CommitContext, RecentCommit, StagedFile};

use log::debug;

const MAX_DIFF_LENGTH: usize = 2000;
const MAX_FILE_CONTENT_LENGTH: usize = 5000;
const MAX_FILES_FOR_DETAILED_CHANGES: usize = 30;

pub fn create_system_prompt(config: &Config) -> anyhow::Result<String> {
    let commit_schema = schemars::schema_for!(GeneratedMessage);
    let commit_schema_str = serde_json::to_string_pretty(&commit_schema)?;

    let combined_instructions = get_combined_instructions(config);
    Ok(format!(
        "# PERSONA\n\
         You are a Principal Linux Kernel Maintainer. You are technically rigorous, demanding, \
         and believe that a commit message is a permanent piece of technical documentation. \
         You expect developers to explain *why* a change is necessary with absolute precision.\n\
         \n\
         # TASK\n\
         Generate a technical commit message for a high-stakes mailing list. The message must \
         provide a clear technical narrative explaining the Problem, Solution, and Reasoning.\n\
         \n\
         # OPERATIONAL GUIDELINES\n\
         \n\
         1. **Technical Justification (The Narrative):**\n\
            - Describe the **Problem**: What is the specific limitation, bug, or missing capability?\n\
            - Describe the **Solution**: How does this patch technically address it?\n\
            - Describe the **Reasoning**: Why is this the correct approach? Mention tradeoffs.\n\
         \n\
         2. **Subsystem Identification:**\n\
            - Use the relevant directory or module as the prefix (e.g., \"core: ...\", \"tui/ui: ...\").\n\
            - The subject line must be imperative and concise.\n\
         \n\
         3. **Tone & Style:**\n\
            - Professional, objective, and authoritative.\n\
            - Use full paragraphs for complex logic. Avoid shallow bullet points.\n\
            - **Negative Constraint:** Avoid generic verbs like \"updated\" or \"fixed\" without context.\n\
         \n\
         4. **Formatting Constraints (STRICT):**\n\
            - **Subject Line:** Maximum 72 characters.\n\
            - **Body Content:** Wrap all lines at exactly 82 characters. This is a hard limit \
            for mailing list compatibility and readability.\n\
         \n\
         # USER INSTRUCTIONS\n\
         {}\n\
         \n\
         # OUTPUT SPECIFICATION\n\
         Your final response MUST be a single, valid JSON object strictly following this schema:\n\
         \n\
         ```json\n\
         {}\n\
         ```\n\
         \n\
         **CRITICAL:** Output ONLY the JSON. No conversational filler.\n",
        combined_instructions, commit_schema_str
    ))
}

pub fn create_user_prompt(context: &CommitContext, detail_level: DetailLevel) -> String {
    let detailed_changes = format_detailed_changes(&context.staged_files);
    let recent_commits = format_recent_commits(&context.recent_commits);
    let staged_changes = format_staged_files(&context.staged_files);
    let author_history = format_enhanced_author_history(&context.author_history, context);

    let detail_instructions = match detail_level {
        DetailLevel::Minimal => "EXIGENCY: Keep it technical and concise. A subsystem subject and a single paragraph of technical reasoning.",
        DetailLevel::Standard => "EXIGENCY: Provide a multi-paragraph technical justification explaining the problem and solution.",
        DetailLevel::Detailed => "EXIGENCY: Exhaustive technical documentation. Explain the state before/after, the logic flow, and architectural implications.",
    };

    format!(
        "### MAINTAINER TASK: GENERATE TECHNICAL COMMIT LOG\n\
         \n\
         #### DATA CONTEXT\n\
         - **Branch:** `{}`\n\
         - **Staged Change List:**\n\
         ```\n\
         {}\n\
         ```\n\
         \n\
         - **Detailed Diffs (Source of Truth):**\n\
         {}\n\
         \n\
         - **Contextual History:**\n\
         {}\n\
         \n\
         - **Detected Style:**\n\
         {}\n\
         \n\
         #### ANALYSIS REQUIREMENTS\n\
         1. **Subsystem Subject:** Determine the most specific subsystem prefix (e.g. \"core\", \"tui/theme\").\n\
         2. **Problem Analysis:** Identify the technical limitation or bug this diff is solving.\n\
         3. **Logic Flow:** Explain the 'How' and 'Why' of the patch implementation.\n\
         \n\
         #### RULES FOR SUCCESS\n\
         - **Subject Line:** format as `<subsystem>: <imperative summary>` (max 72 chars).\n\
         - **Negative Constraint:** NEVER use titles like \"Update file.rs\".\n\
         - **Formatting Constraint:** HARD WRAP all body lines at 82 characters.\n\
         - Focus on the technical merit and the narrative of the change.\n\
         - {}\n\
         \n\
         Generate the JSON object now.",
        context.branch,
        staged_changes,
        detailed_changes,
        recent_commits,
        author_history,
        detail_instructions
    )
}

fn format_recent_commits(commits: &[RecentCommit]) -> String {
    commits
        .iter()
        .map(|commit| {
            format!(
                "{} - {}",
                &commit.hash[..commit.hash.len().min(7)],
                commit.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_staged_files(files: &[StagedFile]) -> String {
    files
        .iter()
        .map(|file| format!("{} - {}", file.path, format_change_type(&file.change_type)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_detailed_changes(files: &[StagedFile]) -> String {
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

    // Limit the number of files in detailed changes to avoid context overflow
    let displayed_files = if files.len() > MAX_FILES_FOR_DETAILED_CHANGES {
        all_sections.push(format!(
            "NOTE: Only first {} files out of {} are shown in detail below.",
            MAX_FILES_FOR_DETAILED_CHANGES,
            files.len()
        ));
        &files[..MAX_FILES_FOR_DETAILED_CHANGES]
    } else {
        files
    };

    // First section: File summaries with diffs
    let diff_section = displayed_files
        .iter()
        .map(|file| {
            let truncated_diff = if file.diff.len() > MAX_DIFF_LENGTH {
                format!(
                    "{}\n... [TRUNCATED {} bytes]",
                    &file.diff[..MAX_DIFF_LENGTH],
                    file.diff.len() - MAX_DIFF_LENGTH
                )
            } else {
                file.diff.clone()
            };

            format!(
                "File: {}\nChange Type: {}\n\nDiff:\n{}",
                file.path,
                format_change_type(&file.change_type),
                truncated_diff
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    all_sections.push(format!(
        "=== DIFFS ({} files) ===\n\n{}",
        displayed_files.len(),
        diff_section
    ));

    // Second section: Full file contents (only for added files)
    let content_files: Vec<_> = displayed_files
        .iter()
        .filter(|file| file.change_type == ChangeType::Added && file.content.is_some())
        .collect();

    if !content_files.is_empty() {
        let content_section = content_files
            .iter()
            .map(|file| {
                let change_indicator = match file.change_type {
                    ChangeType::Added | ChangeType::Deleted => "",
                    ChangeType::Modified => "✏️",
                    ChangeType::Renamed { .. } => "🚚",
                    ChangeType::Copied { .. } => "📋",
                };

                let content = file.content.as_ref().expect("content checked in filter");
                let truncated_content = if content.len() > MAX_FILE_CONTENT_LENGTH {
                    format!(
                        "{}\n... [TRUNCATED {} bytes]",
                        &content[..MAX_FILE_CONTENT_LENGTH],
                        content.len() - MAX_FILE_CONTENT_LENGTH
                    )
                } else {
                    content.clone()
                };

                format!(
                    "{} File: {}\nFull File Content:\n{}\n\n--- End of File ---",
                    change_indicator, file.path, truncated_content
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

fn format_change_type(change_type: &ChangeType) -> String {
    match change_type {
        ChangeType::Added => "Added".to_string(),
        ChangeType::Modified => "Modified".to_string(),
        ChangeType::Deleted => "Deleted".to_string(),
        ChangeType::Renamed { from, .. } => format!("Renamed from {}", from),
        ChangeType::Copied { from, .. } => format!("Copied from {}", from),
    }
}

fn _format_author_history(history: &[String]) -> String {
    if history.is_empty() {
        "No previous commits found for this author.".to_string()
    } else {
        history
            .iter()
            .enumerate()
            .map(|(i, msg)| format!("{}. {}", i + 1, msg.lines().next().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn format_enhanced_author_history(history: &[String], context: &CommitContext) -> String {
    if history.is_empty() {
        "No previous commits found for this author.".to_string()
    } else {
        let conventions = context.detect_conventions();
        let conventions_str = if conventions.is_empty() {
            "No specific conventions detected.".to_string()
        } else {
            format!(
                "Detected conventions: {}",
                conventions
                    .iter()
                    .map(|(k, v)| format!("{} ({} times)", k, v))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        format!(
            "{}\n\n{}",
            conventions_str,
            history
                .iter()
                .enumerate()
                .map(|(i, msg)| format!("{}. {}", i + 1, msg.lines().next().unwrap_or("")))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

fn _format_conventions(conventions: &std::collections::HashMap<String, usize>) -> String {
    if conventions.is_empty() {
        "No specific conventions detected.".to_string()
    } else {
        conventions
            .iter()
            .map(|(k, v)| format!("{} ({} times)", k, v))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub fn create_pr_system_prompt(config: &Config) -> anyhow::Result<String> {
    let pr_schema = schemars::schema_for!(super::types::GeneratedPullRequest);
    let pr_schema_str = serde_json::to_string_pretty(&pr_schema)?;

    let mut prompt = String::from(
        "# PERSONA\n\
        You are a Staff Technical Writer and Senior Developer specialized in documentation and code \
        review. You excel at synthesizing complex code changes into clear, high-level narratives \
        for stakeholders and peer reviewers.\n\
        \n\
        # CORE OBJECTIVE\n\
        Generate a comprehensive, professional Pull Request (PR) description by analyzing the \
        provided commits and diffs as a cohesive unit of work.\n\
        \n\
        # ANALYTICAL PROTOCOL\n\
        1. **Holistic Synthesis:** Do not simply list commits. Identify the single \"Main Theme\" \
        that unifies all changes.\n\
        2. **Value Identification:** Highlight how these changes improve the codebase (performance, \
        stability, features, etc.).\n\
        3. **Technical Depth:** Identify architectural shifts, dependency updates, or API changes.\n\
        4. **Risk Assessment:** Explicitly look for breaking changes or complex logic requiring \
        careful review.\n\
        \n\
        # PR ANATOMY\n\
        - **Title:** Action-oriented and descriptive.\n\
        - **Summary:** The \"TL;DR\" for a busy reviewer.\n\
        - **Description:** Categorized features/fixes with \"What\" and \"Why\" context.\n\
        - **Breaking Changes:** Detailed impact and migration path if applicable.\n\
        - **Testing Notes:** How to verify the work.\n\
        \n\
        # QUALITY GUIDELINES\n\
        - Use professional, active language.\n\
        - Ensure logical grouping of features.\n\
        - Focus on the *intent* behind the changeset.\n\
        ");

    prompt.push_str("\n# ADDITIONAL INSTRUCTIONS\n");
    prompt.push_str(get_combined_instructions(config).as_str());

    prompt.push_str(
        "\n\n# OUTPUT REQUIREMENTS\n\
        Output MUST be a valid JSON object matching the following structure and example logic.\n\
        \n\
        ## SCHEMA\n\
        ```json\n"
    );
    prompt.push_str(&pr_schema_str);
    prompt.push_str("\n```\n\n");

    prompt.push_str(
        "## EXAMPLE LOGIC\n\
        {\n\
          \"title\": \"Add comprehensive Experience Fragment management system\",\n\
          \"summary\": \"Implements full lifecycle support for Experience Fragments (XFs), including create, retrieve, update, and page integration operations.\",\n\
          \"description\": \"### Core Capabilities\\n\\n* Unified `manage_experience_fragments` tool...\",\n\
          \"commits\": [\"b1b1f3f: feat(xf): add experience fragment management system\"],\n\
          \"breaking_changes\": [],\n\
          \"testing_notes\": \"Verified XF creation, update, and population.\",\n\
          \"notes\": \"Requires sufficient AEM permissions.\"\n\
        }\n\
        \n\
        **CRITICAL:** Output ONLY the JSON object. No conversational filler."
    );

    Ok(prompt)
}

pub fn create_completion_system_prompt(config: &Config) -> anyhow::Result<String> {
    let completion_schema = schemars::schema_for!(super::types::GeneratedMessage);
    let completion_schema_str = serde_json::to_string_pretty(&completion_schema)?;

    let combined_instructions = get_combined_instructions(config);
    Ok(format!(
        "# PERSONA\n\
         You are a Git Workflow Expert. You specialize in anticipating a developer's intent \
         and completing their thoughts with precise, idiomatic commit messages.\n\
         \n\
         # TASK\n\
         Complete a partially typed commit message based on the provided code context. \
         Your completion must be a natural continuation that maintains the existing style.\n\
         \n\
         # OPERATIONAL GUIDELINES\n\
         1. **Contextual Continuity:** Analyze the prefix for tone, scope, and convention (e.g., \
         Conventional Commits). Match it exactly.\n\
         2. **Zero Redundancy:** Do not repeat the prefix. Start exactly where the prefix ends.\n\
         3. **Technical Precision:** Use the diffs to ensure the completion accurately reflects \
         the code.\n\
         4. **Formatting:** If the prefix is a title, complete the title (and optionally add a \
         body if appropriate). If the prefix is already in the body, complete the reasoning.\n\
         \n\
         # USER INSTRUCTIONS\n\
         {}\n\
         \n\
         # OUTPUT SPECIFICATION\n\
         Your response must be a valid JSON object matching this schema:\n\
         \n\
         ```json\n\
         {}\n\
         ```\n\
         \n\
         **CRITICAL:** Output ONLY the JSON. No conversational filler.\n",
        combined_instructions, completion_schema_str
    ))
}

pub fn create_completion_user_prompt(
    context: &CommitContext,
    prefix: &str,
    context_ratio: f32,
) -> String {
    let detailed_changes = format_detailed_changes(&context.staged_files);
    let recent_commits = format_recent_commits(&context.recent_commits);
    let staged_changes = format_staged_files(&context.staged_files);
    let author_history = format_enhanced_author_history(&context.author_history, context);

    format!(
        "### TASK: COMPLETE PARTIAL COMMIT MESSAGE\n\
         \n\
         #### USER INPUT\n\
         - **Current Prefix:** `{}`\n\
         - **Context Match Ratio:** {:.0}%\n\
         \n\
         #### DATA CONTEXT\n\
         - **Branch:** `{}`\n\
         - **Staged Files:**\n\
         ```\n\
         {}\n\
         ```\n\
         - **Diff Detais:**\n\
         {}\n\
         - **Recent History:**\n\
         {}\n\
         - **Author Style:**\n\
         {}\n\
         \n\
         #### COMPLETION INSTRUCTIONS\n\
         1. **Syntactic Match:** If the prefix ends with a colon or a space, continue with the \
         description. If it ends mid-word, finish the word.\n\
         2. **Pattern Recognition:** Use the author's history to determine the likely completion.\n\
         3. **Final synthesis:** The final message (Prefix + your Completion) must be a high-quality, \
         professional commit message.\n\
         \n\
         Generate the JSON completion now.",
        prefix,
        context_ratio * 100.0,
        context.branch,
        staged_changes,
        detailed_changes,
        recent_commits,
        author_history
    )
}

/// Creates a user prompt for PR description generation
pub fn create_pr_user_prompt(context: &CommitContext, commit_messages: &[String]) -> String {
    let detailed_changes = format_detailed_changes(&context.staged_files);

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
         Detailed changes:\n{}",
        context.branch,
        commits_section,
        format_recent_commits(&context.recent_commits),
        format_staged_files(&context.staged_files),
        detailed_changes
    );

    debug!(
        "Generated PR prompt for {} files across {} commits",
        context.staged_files.len(),
        commit_messages.len()
    );

    prompt
}
