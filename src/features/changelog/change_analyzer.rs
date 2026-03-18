use super::engine::{ChangeAnalysisEngine, DefaultAnalysisEngine};
use super::models::{ChangeMetrics, ChangelogType};
use crate::core::context::{ChangeType, RecentCommit};
use crate::git::GitRepo;

use anyhow::Result;
use git2::Oid;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Represents the analyzed changes for a single commit
#[derive(Debug, Clone)]
pub struct AnalyzedChange {
    pub commit_hash: String,
    pub commit_message: String,
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
    engine: Box<dyn ChangeAnalysisEngine>,
}

impl ChangeAnalyzer {
    /// Create a new `ChangeAnalyzer` instance
    pub fn new(git_repo: Arc<GitRepo>) -> Result<Self> {
        Ok(Self {
            git_repo,
            engine: Box::new(DefaultAnalysisEngine),
        })
    }

    /// Set a custom analysis engine
    #[must_use]
    pub fn with_engine(mut self, engine: Box<dyn ChangeAnalysisEngine>) -> Self {
        self.engine = engine;
        self
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

        // Since we need to use self.engine in the blocking task, we wrap the analyzer in Arc
        // or just pass the engine if it's clonable. But trait objects are tricky.
        // For now, we'll re-create the default engine or pass the Arc'd engine.
        let engine = Arc::new(DefaultAnalysisEngine); // Currently DefaultAnalysisEngine is stateless

        let _ = tokio::task::spawn_blocking(move || {
            git_repo.get_commits_between_stream(&from, &to, |commit| {
                let analyzed = Self::analyze_commit_inner(&git_repo, engine.as_ref(), commit)?;
                let _ = tx.blocking_send(Ok(analyzed));
                Ok(())
            })
        })
        .await?;
        Ok(())
    }

    /// Analyze a single commit (blocking)
    fn analyze_commit_inner(
        git_repo: &GitRepo,
        engine: &dyn ChangeAnalysisEngine,
        commit: &RecentCommit,
    ) -> Result<AnalyzedChange> {
        let repo = git_repo.open_repo()?;
        let commit_obj = repo.find_commit(Oid::from_str(&commit.hash)?)?;

        let parent_tree = if commit_obj.parent_count() > 0 {
            Some(commit_obj.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_obj.tree()?), None)?;

        let file_changes = engine.analyze_file_changes(&diff)?;
        let metrics = engine.calculate_metrics(&diff)?;
        let change_type = engine.classify_change(&commit.message, &file_changes);
        let is_breaking_change = engine.detect_breaking_change(&commit.message, &file_changes);
        let associated_issues = engine.extract_associated_issues(&commit.message);
        let pull_request = engine.extract_pull_request(&commit.message);
        let impact_score =
            Self::calculate_impact_score(&metrics, &file_changes, is_breaking_change);

        Ok(AnalyzedChange {
            commit_hash: commit.hash.clone(),
            commit_message: commit.message.clone(),
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
