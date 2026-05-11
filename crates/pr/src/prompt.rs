use crate::models::GeneratedPullRequest;
use claw_core::common::get_combined_instructions;
use claw_core::config::Config;
use claw_core::llm::context::{ChangeType, CommitContext};

const MAX_DIFF_LENGTH: usize = 2000;
const MAX_FILE_CONTENT_LENGTH: usize = 5000;
const MAX_FILES_FOR_DETAILED_CHANGES: usize = 30;

pub fn create_pr_system_prompt(config: &Config) -> anyhow::Result<String> {
    let pr_schema = schemars::schema_for!(GeneratedPullRequest);
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
          4. **Handling Partial Information:**\n\
             - Do not speculate on the contents of the truncated portions; instead, infer the \
             overall architectural intent from the visible hunks and the file names.\n\
          \n\
          5. **Formatting Constraints:**\n\
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

fn format_recent_commits(commits: &[claw_core::llm::context::RecentCommit]) -> String {
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

fn format_detailed_changes(files: &[claw_core::llm::context::StagedFile]) -> String {
    let mut all_sections = Vec::new();

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

    let diff_section = displayed_files
        .iter()
        .map(|file| {
            let truncated_diff = truncate_smartly(&file.diff, MAX_DIFF_LENGTH);

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

    let content_files: Vec<_> = displayed_files
        .iter()
        .filter(|file| file.change_type == ChangeType::Added && file.content.is_some())
        .collect();

    if !content_files.is_empty() {
        let content_section = content_files
            .iter()
            .filter_map(|file| {
                let content = file.content.as_ref()?;
                let truncated_content = truncate_smartly(content, MAX_FILE_CONTENT_LENGTH);
                Some(format!(
                    "File: {}\nFull File Content:\n{}\n\n--- End of File ---",
                    file.path, truncated_content
                ))
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
        ChangeType::Renamed { from, .. } => format!("Renamed from {from}"),
        ChangeType::Copied { from, .. } => format!("Copied from {from}"),
    }
}

fn truncate_smartly(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }

    let mut result = String::with_capacity(max_len + 50);
    for line in text.lines() {
        result.push_str(line);
        result.push('\n');
    }

    result
}
