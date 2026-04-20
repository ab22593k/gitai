use super::change_analyzer::FileChange;
use super::models::{ChangeMetrics, ChangelogType};
use crate::llm::context::ChangeType;
use anyhow::Result;
use git2::Diff;
use regex::Regex;

// Regex for extracting issue numbers (e.g., #123, GH-123)
static ISSUE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?:#|GH-)(\d+)")
        .expect("Failed to compile issue number regex pattern - this is a bug")
});

// Regex for extracting pull request numbers (e.g., PR #123, pull request 123)
static PR_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?i)(?:pull request|PR)\s*#?(\d+)")
        .expect("Failed to compile pull request regex pattern - this is a bug")
});

/// Trait for defining the logic to analyze code changes
pub trait ChangeAnalysisEngine: Send + Sync {
    /// Analyze changes for each file in the diff
    fn analyze_file_changes(&self, diff: &Diff) -> Result<Vec<FileChange>>;

    /// Calculate metrics for the diff
    fn calculate_metrics(&self, diff: &Diff) -> Result<ChangeMetrics>;

    /// Classify the type of change based on commit message and file changes
    fn classify_change(&self, message: &str, file_changes: &[FileChange]) -> ChangelogType;

    /// Detect if the change is a breaking change
    fn detect_breaking_change(&self, message: &str, file_changes: &[FileChange]) -> bool;

    /// Extract associated issue numbers from the commit message
    fn extract_associated_issues(&self, message: &str) -> Vec<String>;

    /// Extract pull request number from the commit message
    fn extract_pull_request(&self, message: &str) -> Option<String>;
}

/// Default implementation of the change analysis engine
pub struct DefaultAnalysisEngine;

impl ChangeAnalysisEngine for DefaultAnalysisEngine {
    fn analyze_file_changes(&self, diff: &Diff) -> Result<Vec<FileChange>> {
        let mut file_changes = Vec::new();

        diff.foreach(
            &mut |delta, _| {
                let old_file = delta.old_file();
                let new_file = delta.new_file();
                let change_type = match delta.status() {
                    git2::Delta::Added => ChangeType::Added,
                    git2::Delta::Deleted => ChangeType::Deleted,
                    _ => ChangeType::Modified,
                };

                let file_path = new_file.path().map_or_else(
                    || {
                        old_file
                            .path().map_or_else(|| {
                                log::debug!("DiffDelta has neither old nor new file path");
                                String::new()
                            }, |p| p.to_string_lossy().into_owned())
                    },
                    |p| p.to_string_lossy().into_owned(),
                );

                // Perform file-specific analysis based on file type
                let mut analysis = Vec::new();

                // Determine file type and add relevant analysis
                if let Some(extension) = std::path::Path::new(&file_path).extension()
                    && let Some(ext_str) = extension.to_str()
                {
                    match ext_str.to_lowercase().as_str() {
                        "rs" => analysis.push("Rust source code changes".to_string()),
                        "js" | "ts" => {
                            analysis.push("JavaScript/TypeScript changes".to_string());
                        }
                        "py" => analysis.push("Python code changes".to_string()),
                        "java" => analysis.push("Java code changes".to_string()),
                        "c" | "cpp" | "h" => analysis.push("C/C++ code changes".to_string()),
                        "md" => analysis.push("Documentation changes".to_string()),
                        "json" | "yml" | "yaml" | "toml" => {
                            analysis.push("Configuration changes".to_string());
                        }
                        _ => {}
                    }
                }

                // Add analysis based on change type
                match &change_type {
                    ChangeType::Added => analysis.push("New file added".to_string()),
                    ChangeType::Deleted => analysis.push("File removed".to_string()),
                    ChangeType::Renamed { from, .. } => {
                        analysis.push(format!("File renamed from {}", from));
                    }
                    ChangeType::Copied { from, .. } => {
                        analysis.push(format!("File copied from {}", from));
                    }
                    ChangeType::Modified => {
                        if file_path.contains("test") || file_path.contains("spec") {
                            analysis.push("Test modifications".to_string());
                        } else if file_path.contains("README") || file_path.contains("docs/") {
                            analysis.push("Documentation updates".to_string());
                        }
                    }
                }

                let old_path = old_file
                    .path().map_or_else(|| {
                        log::debug!("DiffDelta missing old file path for {:?}", delta.status());
                        String::new()
                    }, |p| p.to_string_lossy().into_owned());
                let new_path = new_file
                    .path().map_or_else(|| {
                        log::debug!("DiffDelta missing new file path for {:?}", delta.status());
                        String::new()
                    }, |p| p.to_string_lossy().into_owned());

                let file_change = FileChange {
                    old_path,
                    new_path,
                    change_type,
                    analysis,
                };

                file_changes.push(file_change);
                true
            },
            None,
            None,
            None,
        )?;

        Ok(file_changes)
    }

    fn calculate_metrics(&self, diff: &Diff) -> Result<ChangeMetrics> {
        let stats = diff.stats()?;
        Ok(ChangeMetrics {
            total_commits: 1,
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
            total_lines_changed: stats.insertions() + stats.deletions(),
        })
    }

    fn classify_change(&self, message: &str, file_changes: &[FileChange]) -> ChangelogType {
        let message_lower = message.to_lowercase();

        // First, check the commit message
        if message_lower.contains("add") || message_lower.contains("new") {
            ChangelogType::Added
        } else if message_lower.contains("deprecat") {
            ChangelogType::Deprecated
        } else if message_lower.contains("remov") || message_lower.contains("delet") {
            ChangelogType::Removed
        } else if message_lower.contains("fix") || message_lower.contains("bug") {
            ChangelogType::Fixed
        } else if message_lower.contains("secur") || message_lower.contains("vulnerab") {
            ChangelogType::Security
        } else {
            // If the commit message doesn't give us a clear indication, check the file changes
            let has_additions = file_changes
                .iter()
                .any(|fc| fc.change_type == ChangeType::Added);
            let has_deletions = file_changes
                .iter()
                .any(|fc| fc.change_type == ChangeType::Deleted);

            if has_additions && !has_deletions {
                ChangelogType::Added
            } else if has_deletions && !has_additions {
                ChangelogType::Removed
            } else {
                ChangelogType::Changed
            }
        }
    }

    fn detect_breaking_change(&self, message: &str, file_changes: &[FileChange]) -> bool {
        let message_lower = message.to_lowercase();
        if message_lower.contains("breaking change")
            || message_lower.contains("breaking-change")
            || message_lower.contains("major version")
        {
            return true;
        }

        // Check file changes for potential breaking changes
        file_changes.iter().any(|fc| {
            fc.analysis.iter().any(|analysis| {
                let analysis_lower = analysis.to_lowercase();
                analysis_lower.contains("breaking change")
                    || analysis_lower.contains("api change")
                    || analysis_lower.contains("incompatible")
            })
        })
    }

    fn extract_associated_issues(&self, message: &str) -> Vec<String> {
        ISSUE_RE
            .captures_iter(message)
            .map(|cap| format!("#{}", &cap[1]))
            .collect()
    }

    fn extract_pull_request(&self, message: &str) -> Option<String> {
        PR_RE
            .captures(message)
            .map(|cap| format!("PR #{}", &cap[1]))
    }
}
