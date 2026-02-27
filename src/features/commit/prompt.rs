use super::types::GeneratedMessage;
use crate::common::{DetailLevel, get_combined_instructions};
use crate::config::Config;
use crate::core::context::{ChangeType, CommitContext, RecentCommit, StagedFile};

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
        DetailLevel::Minimal => {
            "EXIGENCY: Keep it technical and concise. A subsystem subject and a single paragraph of technical reasoning."
        }
        DetailLevel::Standard => {
            "EXIGENCY: Provide a multi-paragraph technical justification explaining the problem and solution."
        }
        DetailLevel::Detailed => {
            "EXIGENCY: Exhaustive technical documentation. Explain the state before/after, the logic flow, and architectural implications."
        }
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

    let combined_instructions = get_combined_instructions(config);
    Ok(format!(
        "# PERSONA\n\
         You are a Principal Linux Kernel Maintainer. You are technically rigorous, demanding, \
         and believe that a PR description (cover letter) is a permanent piece of technical \
         documentation for the project's history. You expect developers to justify their \
         architectural choices with absolute precision.\n\
         \n\
         # CORE OBJECTIVE\n\
         Generate a comprehensive, professional technical narrative for a high-stakes pull request. \
         Analyze the provided commits and diffs as a cohesive unit of work, not just a list of \
         changes.\n\
         \n\
         # OPERATIONAL GUIDELINES\n\
         1. **Technical Narrative (The Cover Letter Style):**\n\
            - Describe the **Context**: What subsystem or capability is being modified?\n\
            - Describe the **Problem**: What is the specific limitation, bug, or missing feature?\n\
            - Describe the **Solution**: How does this changeset technically address the problem?\n\
            - Describe the **Reasoning**: Why is this the correct approach? Mention tradeoffs, \
            alternatives considered, and architectural impact.\n\
         \n\
         2. **Subsystem Identification:**\n\
            - Identify the primary subsystem being touched (e.g., \"core\", \"tui\", \"git\").\n\
            - The title should be imperative and follow the \"subsystem: summary\" pattern.\n\
         \n\
         3. **Tone & Style:**\n\
            - Professional, objective, and authoritative.\n\
            - Avoid \"shallow\" bullet points for complex logic; use full, technical paragraphs.\n\
            - Ensure the intent behind the changeset is crystalline.\n\
         \n\
         4. **Formatting Constraints:**\n\
            - Wrap all body text at exactly 82 characters for maximum readability in diff-friendly \
            environments.\n\
         \n\
         # USER INSTRUCTIONS\n\
         {}\n\
         \n\
         # OUTPUT SPECIFICATION\n\
         Your final response MUST be a single, valid JSON object matching this schema:\n\
         \n\
         ```json\n\
         {}\n\
         ```\n\
         \n\
         **CRITICAL:** Output ONLY the JSON object. No conversational filler.",
        combined_instructions, pr_schema_str
    ))
}

/// Creates a user prompt for PR description generation
pub fn create_pr_user_prompt(context: &CommitContext, commit_messages: &[String]) -> String {
    let detailed_changes = format_detailed_changes(&context.staged_files);
    let recent_commits = format_recent_commits(&context.recent_commits);

    let commits_section = if commit_messages.is_empty() {
        "No commits in current range.".to_string()
    } else {
        commit_messages.join("\n")
    };

    format!(
        "### MAINTAINER TASK: GENERATE PR TECHNICAL NARRATIVE\n\
         \n\
         #### DATA CONTEXT\n\
         - **Branch/Range:** `{}`\n\
         \n\
         - **Commits to Analyze (Current Work):**\n\
         ```\n\
         {}\n\
         ```\n\
         \n\
         - **Detailed Diffs (Source of Truth):**\n\
         {}\n\
         \n\
         - **Contextual Project History:**\n\
         {}\n\
         \n\
         #### ANALYSIS REQUIREMENTS\n\
         1. **Subsystem Context:** Identify the core module being evolved.\n\
         2. **Change Rationale:** Extract the 'Why' from the commits and diffs.\n\
         3. **Impact Assessment:** Determine what changed for the system and the user.\n\
         \n\
         #### RULES FOR SUCCESS\n\
         - Use the \"Problem / Solution / Reasoning\" structure in the description field.\n\
         - Ensure the title is formatted as `<subsystem>: <short description>`.\n\
         - HARD WRAP all body lines at 82 characters.\n\
         \n\
         Generate the JSON PR description now.",
        context.branch, commits_section, detailed_changes, recent_commits
    )
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
