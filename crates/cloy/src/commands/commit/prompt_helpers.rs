use crate::llm::context::{ChangeType, CommitContext, RecentCommit, StagedFile};

const MAX_DIFF_LENGTH: usize = 2000;
const MAX_FILE_CONTENT_LENGTH: usize = 5000;
const MAX_FILES_FOR_DETAILED_CHANGES: usize = 30;

pub fn format_recent_commits(commits: &[RecentCommit]) -> String {
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

pub fn format_staged_files(files: &[StagedFile]) -> String {
    files
        .iter()
        .map(|file| format!("{} - {}", file.path, format_change_type(&file.change_type)))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_detailed_changes(files: &[StagedFile]) -> String {
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
                let change_indicator = match file.change_type {
                    ChangeType::Added | ChangeType::Deleted => "",
                    ChangeType::Modified => "✏️",
                    ChangeType::Renamed { .. } => "🚚",
                    ChangeType::Copied { .. } => "📋",
                };

                let content = file.content.as_ref()?;
                let truncated_content = truncate_smartly(content, MAX_FILE_CONTENT_LENGTH);
                Some(format!(
                    "{} File: {}\nFull File Content:\n{}\n\n--- End of File ---",
                    change_indicator, file.path, truncated_content
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

pub fn format_enhanced_author_history(history: &[String], context: &CommitContext) -> String {
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
                    .map(|(k, v)| format!("{k} ({v} times)"))
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
