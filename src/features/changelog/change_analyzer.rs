use super::models::{ChangeMetrics, ChangelogType};
use crate::core::context::{ChangeType, RecentCommit};
use crate::git::GitRepo;

use anyhow::Result;
use git2::{Diff, Oid};
use regex::Regex;
use std::sync::Arc;
use tokio::sync::mpsc;

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

/// Represents the analyzed changes for a single commit
#[derive(Debug, Clone)]
pub struct AnalyzedChange {
    pub commit_hash: String,
    pub commit_message: String,
    pub author: String,
    pub file_changes: Vec<FileChange>,
    pub metrics: ChangeMetrics,
    pub impact_score: f32,
    pub change_type: ChangelogType,
    pub is_breaking_change: bool,
    pub associated_issues: Vec<String>,
    pub pull_request: Option<String>,
}

/// Represents changes to a single file
#[derive(Debug, Clone)]
pub struct FileChange {
    pub old_path: String,
    pub new_path: String,
    pub change_type: ChangeType,
    pub analysis: Vec<String>,
}

/// Analyzer for processing Git commits and generating detailed change information
pub struct ChangeAnalyzer {
    git_repo: Arc<GitRepo>,
}

impl ChangeAnalyzer {
    /// Create a new `ChangeAnalyzer` instance
    pub fn new(git_repo: Arc<GitRepo>) -> Result<Self> {
        Ok(Self { git_repo })
    }

    /// Analyze commits between two Git references, streaming results via channel
    pub async fn analyze_commits(
        &self,
        from: &str,
        to: &str,
        tx: mpsc::Sender<Result<AnalyzedChange>>,
    ) -> Result<()> {
        let git_repo = self.git_repo.clone();
        let from = from.to_string();
        let to = to.to_string();
        let _ = tokio::task::spawn_blocking(move || {
            git_repo.get_commits_between_stream(&from, &to, |commit| {
                let analyzed = Self::analyze_commit_inner(&git_repo, commit)?;
                let _ = tx.blocking_send(Ok(analyzed));
                Ok(())
            })
        })
        .await?;
        Ok(())
    }

    /// Analyze a single commit (blocking)
    fn analyze_commit_inner(git_repo: &GitRepo, commit: &RecentCommit) -> Result<AnalyzedChange> {
        let repo = git_repo.open_repo()?;
        let commit_obj = repo.find_commit(Oid::from_str(&commit.hash)?)?;

        let parent_tree = if commit_obj.parent_count() > 0 {
            Some(commit_obj.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_obj.tree()?), None)?;

        let file_changes = Self::analyze_file_changes(&diff)?;
        let metrics = Self::calculate_metrics(&diff)?;
        let change_type = Self::classify_change(&commit.message, &file_changes);
        let is_breaking_change = Self::detect_breaking_change(&commit.message, &file_changes);
        let associated_issues = Self::extract_associated_issues(&commit.message);
        let pull_request = Self::extract_pull_request(&commit.message);
        let impact_score =
            Self::calculate_impact_score(&metrics, &file_changes, is_breaking_change);

        Ok(AnalyzedChange {
            commit_hash: commit.hash.clone(),
            commit_message: commit.message.clone(),
            author: commit.author.clone(),
            file_changes,
            metrics,
            impact_score,
            change_type,
            is_breaking_change,
            associated_issues,
            pull_request,
        })
    }

    /// Analyze changes between two Git references and return the analyzed changes along with total metrics
    pub async fn analyze_changes(
        &self,
        from: &str,
        to: &str,
    ) -> Result<(Vec<AnalyzedChange>, ChangeMetrics)> {
        let (tx, mut rx) = mpsc::channel(100);
        let analyze_task = self.analyze_commits(from, to, tx);
        let collect_task = async {
            let mut analyzed_changes = Vec::new();
            while let Some(result) = rx.recv().await {
                analyzed_changes.push(result?);
            }
            Ok(analyzed_changes)
        };
        let ((), analyzed_changes) = tokio::try_join!(analyze_task, collect_task)?;
        let total_metrics = self.calculate_total_metrics(&analyzed_changes);
        Ok((analyzed_changes, total_metrics))
    }

    /// Analyze changes for each file in the commit
    fn analyze_file_changes(diff: &Diff) -> Result<Vec<FileChange>> {
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
                            .path()
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_default()
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

                let file_change = FileChange {
                    old_path: old_file
                        .path()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    new_path: new_file
                        .path()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
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

    /// Calculate metrics for the commit
    fn calculate_metrics(diff: &Diff) -> Result<ChangeMetrics> {
        let stats = diff.stats()?;
        Ok(ChangeMetrics {
            total_commits: 1,
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
            total_lines_changed: stats.insertions() + stats.deletions(),
        })
    }

    /// Classify the type of change based on commit message and file changes
    fn classify_change(commit_message: &str, file_changes: &[FileChange]) -> ChangelogType {
        let message_lower = commit_message.to_lowercase();

        // First, check the commit message
        if message_lower.contains("add") || message_lower.contains("new") {
            return ChangelogType::Added;
        } else if message_lower.contains("deprecat") {
            return ChangelogType::Deprecated;
        } else if message_lower.contains("remov") || message_lower.contains("delet") {
            return ChangelogType::Removed;
        } else if message_lower.contains("fix") || message_lower.contains("bug") {
            return ChangelogType::Fixed;
        } else if message_lower.contains("secur") || message_lower.contains("vulnerab") {
            return ChangelogType::Security;
        }

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

    /// Detect if the change is a breaking change
    fn detect_breaking_change(commit_message: &str, file_changes: &[FileChange]) -> bool {
        let message_lower = commit_message.to_lowercase();
        if message_lower.contains("breaking change")
            || message_lower.contains("breaking-change")
            || message_lower.contains("major version")
        {
            return true;
        }

        // Check file changes for potential breaking changes
        file_changes.iter().any(|fc| {
            fc.analysis.iter().any(|analysis| {
                analysis.to_lowercase().contains("breaking change")
                    || analysis.to_lowercase().contains("api change")
                    || analysis.to_lowercase().contains("incompatible")
            })
        })
    }

    /// Extract associated issue numbers from the commit message
    fn extract_associated_issues(commit_message: &str) -> Vec<String> {
        // Use the lazily initialized static regex
        ISSUE_RE
            .captures_iter(commit_message)
            .map(|cap| format!("#{}", &cap[1]))
            .collect()
    }

    /// Extract pull request number from the commit message
    fn extract_pull_request(commit_message: &str) -> Option<String> {
        // Use the lazily initialized static regex
        PR_RE
            .captures(commit_message)
            .map(|cap| format!("PR #{}", &cap[1]))
    }

    /// Calculate the impact score of the change
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::as_conversions)]
    fn calculate_impact_score(
        metrics: &ChangeMetrics,
        file_changes: &[FileChange],
        is_breaking_change: bool,
    ) -> f32 {
        let base_score = (metrics.total_lines_changed as f32) / 100.0;
        let file_score = file_changes.len() as f32 / 10.0;
        let breaking_change_score = if is_breaking_change { 5.0 } else { 0.0 };

        base_score + file_score + breaking_change_score
    }

    /// Calculate total metrics for a set of analyzed changes
    pub fn calculate_total_metrics(&self, changes: &[AnalyzedChange]) -> ChangeMetrics {
        changes.iter().fold(
            ChangeMetrics {
                total_commits: changes.len(),
                files_changed: 0,
                insertions: 0,
                deletions: 0,
                total_lines_changed: 0,
            },
            |mut acc, change| {
                acc.files_changed += change.metrics.files_changed;
                acc.insertions += change.metrics.insertions;
                acc.deletions += change.metrics.deletions;
                acc.total_lines_changed += change.metrics.total_lines_changed;
                acc
            },
        )
    }
}
