use super::types::GeneratedMessage;
use crate::common::{DetailLevel, get_combined_instructions};
use crate::config::Config;
use crate::core::context::{ChangeType, CommitContext, RecentCommit, StagedFile};

use log::debug;

pub fn create_system_prompt(config: &Config) -> anyhow::Result<String> {
    let commit_schema = schemars::schema_for!(GeneratedMessage);
    let commit_schema_str = serde_json::to_string_pretty(&commit_schema)?;

    let combined_instructions = get_combined_instructions(config);
    Ok(format!(
        "# ROLE: Software Engineer\n\
         \n\
         You are an expert Quality Assurance (QA) Engineer specializing in creating high-quality, \
         conventional commit messages from code changes.\n\
         \n\
         ## Core Responsibilities\n\
         \n\
         1. **Analyze Context:** Infer the intent and impact of code changes\n\
         2. **Generate Messages:** Create well-structured, conventional commit messages\n\
         3. **Maintain Standards:** Follow conventional commit format and best practices\n\
         4. **Ensure Quality:** Make messages concise, descriptive, and actionable\n\
         \n\
         ## Instructions\n\
         \n\
         {}\n\
         \n\
         ## Output Requirements\n\
         \n\
         **Format:** Your final output MUST STRICTLY conform to the following JSON schema:\n\
         \n\
         ```json\n\
         {}\n\
         ```\n\
         \n\
         **Important:** Output ONLY the JSON object. No explanatory text, preambles, \
         or additional content.\n",
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
            "4. Message should be EXTREMELY concise. Generate ONLY a single title line if possible, or a title and one short summary line. No long bullet points."
        }
        DetailLevel::Standard => {
            "4. Message should be concise yet descriptive. Include a title and a brief summary."
        }
        DetailLevel::Detailed => {
            "4. Provide a detailed explanation. Include a title, comprehensive summary, and detailed bullet points explaining the changes."
        }
    };

    debug!(
        "Generated commit prompt for {} files ({} added, {} modified, {} deleted) with detail level: {:?}",
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
            .count(),
        detail_level
    );

    format!(
        "# TASK: Generate Commit Message\n\
         \n\
         ANALYZE the provided context and generate a well-structured commit message.\n\
         \n\
         ## Context Information\n\
         \n\
         **Branch:** {}\n\
         \n\
         **Detailed Changes (Diffs):**\n\
         {}\n\
         \n\
         **Recent Commits (for changed files):**\n\
         {}\n\
         \n\
         **Staged Changes List:**\n\
         {}\n\
         \n\
         **Author's Commit History:**\n\
         {}\n\
         \n\
         ## Analysis Requirements\n\
         \n\
         1. **PRIMARY FOCUS:** Analyze the 'Detailed Changes' section. Use the diffs as the source of truth.\n\
          Use the following format:\n\
        - First line: A concise summary using conventional commits format (type: description) where appropriate\n\
        - Leave a blank line after the first line\n\
        - Then add 2-3 bullet points explaining the key changes\n\
        - Focus on WHAT changed and WHY, not HOW.\n\
        - Return ONLY the commit message without any additional text.
         {}\n\
         6. Focus on the intent and impact of the changes, ignoring large boilerplate updates if trivial.\n",
        context.branch,
        detailed_changes,
        recent_commits,
        staged_changes,
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

    // First section: File summaries with diffs
    let diff_section = files
        .iter()
        .map(|file| {
            format!(
                "File: {}\nChange Type: {}\n\nDiff:\n{}",
                file.path,
                format_change_type(&file.change_type),
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

    // Second section: Full file contents (only for added files)
    let content_files: Vec<_> = files
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

/// Creates a system prompt for PR description generation
pub fn create_pr_system_prompt(config: &Config) -> anyhow::Result<String> {
    let pr_schema = schemars::schema_for!(super::types::GeneratedPullRequest);
    let pr_schema_str = serde_json::to_string_pretty(&pr_schema)?;

    let mut prompt = String::from(
        "# ROLE: Pull Request Description Specialist\n\
        \n\
        You are an AI assistant specializing in generating comprehensive, professional pull request descriptions. \
        Your task is to ANALYZE the provided context and create clear, informative, and well-structured PR descriptions.\n\
        \n\
        ## Core Responsibilities\n\
        \n\
        1. **Analyze Changeset:** Understand the overall purpose and impact of the PR as a cohesive unit\n\
        2. **Create Structure:** Generate well-organized PR descriptions with clear sections\n\
        3. **Explain Context:** Focus on the 'why' and 'how' behind changes, not just the 'what'\n\
        4. **Ensure Quality:** Use professional language suitable for code review\n\
        \n\
        ## PR Description Structure\n\
        \n\
        ANALYZE the commits and changes, then create:\n\
        \n\
        1. **Title:** Concise, descriptive title summarizing the main changes\n\
        2. **Summary:** Brief overview capturing the essence of what was changed\n\
        3. **Description:** Detailed explanation of changes, rationale, and implementation\n\
        4. **Commits:** List of all commits included in this PR for reference\n\
        5. **Breaking Changes:** Identify and explain any breaking changes\n\
        6. **Testing:** Provide testing instructions if specific steps are required\n\
        7. **Additional Notes:** Include helpful context for reviewers\n\
        \n\
        ## Guidelines\n\
        \n\
        - **Holistic View:** Treat the changeset as an atomic unit, not individual commits\n\
        - **Clear Language:** Use professional, accessible language for all developers\n\
        - **Logical Organization:** Structure information with proper sections and hierarchy\n\
        - **Comprehensive Yet Concise:** Be thorough but avoid unnecessary detail\n\
        - **Reviewer-Focused:** Consider what reviewers need to understand and approve\n\
        - **Technical Context:** Highlight configuration, migration, deployment, and performance considerations\n\
        - **Dependencies:** Mention prerequisites, dependencies, and architectural decisions\n\
        \n\
        Focus on the overall impact and purpose rather than individual commit details.
        ");

    prompt.push_str(get_combined_instructions(config).as_str());

    prompt.push_str(
        "
        Your response must be a valid JSON object with the following structure:

        {
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

/// Creates a system prompt for commit message completion
pub fn create_completion_system_prompt(config: &Config) -> anyhow::Result<String> {
    let completion_schema = schemars::schema_for!(super::types::GeneratedMessage);
    let completion_schema_str = serde_json::to_string_pretty(&completion_schema)?;

    let combined_instructions = get_combined_instructions(config);
    Ok(format!(
        "# ROLE: Git Commit Message Completion Specialist\n\
         \n\
         You are an expert Git Commit Message Completion Specialist specializing in completing \
         partially typed commit messages with high-quality, contextually appropriate continuations.\n\
         \n\
         ## Core Responsibilities\n\
         \n\
         1. **Complete Messages:** Finish partially typed commit messages naturally\n\
         2. **Maintain Context:** Preserve the original intent and style of the prefix\n\
         3. **Ensure Quality:** Create coherent, well-structured final messages\n\
         4. **Follow Standards:** Use conventional commit format when appropriate\n\
         \n\
         ## Instructions\n\
         \n\
         {}\n\
         \n\
         ## Completion Rules\n\
         \n\
         1. **Start Point:** Begin completion exactly where the prefix ends\n\
         2. **Style Consistency:** Maintain the same tone, style, and conventions as the prefix\n\
         3. **Natural Flow:** Complete the message naturally without repeating the prefix\n\
         4. **Technical Accuracy:** Focus on technical accuracy and clarity\n\
         5. **Format Standards:** Use conventional commit format when appropriate\n\
         \n\
         ## Output Requirements\n\
         \n\
         **Format:** Your final output MUST STRICTLY conform to the following JSON schema:\n\
         \n\
         ```json\n\
         {}\n\
         ```\n\
         \n\
         **Important:** Output ONLY the JSON object. No explanatory text, preambles, \
         or additional content.\n",
        combined_instructions, completion_schema_str
    ))
}

/// Creates a user prompt for commit message completion
pub fn create_completion_user_prompt(
    context: &CommitContext,
    prefix: &str,
    context_ratio: f32,
) -> String {
    let detailed_changes = format_detailed_changes(&context.staged_files);

    let recent_commits = format_recent_commits(&context.recent_commits);
    let staged_changes = format_staged_files(&context.staged_files);
    let author_history = format_enhanced_author_history(&context.author_history, context);

    // Detect conventions from history (already included in enhanced author history)

    debug!(
        "Generated completion prompt for {} files ({} added, {} modified, {} deleted), prefix: '{}', context_ratio: {:.2}",
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
            .count(),
        prefix,
        context_ratio
    );

    format!(
        "# TASK: Complete Commit Message\n\
         \n\
         ANALYZE the provided context and complete the commit message naturally.\n\
         \n\
         ## Message Prefix\n\
         \n\
         **Prefix:** '{}'\n\
         **Context Ratio:** {:.0}%\n\
         \n\
         ## Context Information\n\
         \n\
         **Branch:** {}\n\
         \n\
         **Recent Commits (for changed files):**\n\
         {}\n\
         \n\
         **Staged Changes:**\n\
         {}\n\
         \n\
         **Detailed Changes:**\n\
         {}\n\
         \n\
         **Author's Commit History:**\n\
         {}\n\
         \n\
         ## Completion Requirements\n\
         \n\
         1. ANALYZE the author's commit history patterns\n\
         2. Complete the message maintaining the same style and conventions as the prefix\n\
         3. Continue naturally from where the prefix ends\n\
         4. Ensure the completed message is coherent and well-structured\n\
         5. Follow conventional commit standards when appropriate\n",
        prefix,
        context_ratio * 100.0,
        context.branch,
        recent_commits,
        staged_changes,
        detailed_changes,
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
