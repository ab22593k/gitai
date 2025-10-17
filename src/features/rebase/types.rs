//! Types for rebase operations

use serde::{Deserialize, Serialize};

/// Actions that can be performed on commits during rebase
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebaseAction {
    /// Keep the commit as-is
    Pick,
    /// Change the commit message
    Reword,
    /// Stop for manual editing of the commit
    Edit,
    /// Combine this commit with the previous one, keeping both messages
    Squash,
    /// Combine this commit with the previous one, keeping only the previous message
    Fixup,
    /// Remove this commit entirely
    Drop,
}

impl std::fmt::Display for RebaseAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RebaseAction::Pick => write!(f, "pick"),
            RebaseAction::Reword => write!(f, "reword"),
            RebaseAction::Edit => write!(f, "edit"),
            RebaseAction::Squash => write!(f, "squash"),
            RebaseAction::Fixup => write!(f, "fixup"),
            RebaseAction::Drop => write!(f, "drop"),
        }
    }
}

/// A commit in the rebase operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebaseCommit {
    /// Commit hash
    pub hash: String,
    /// Commit message
    pub message: String,
    /// Author name
    pub author: String,
    /// Commit date
    pub date: String,
    /// AI-suggested action for this commit
    pub suggested_action: RebaseAction,
    /// Confidence score for the suggestion (0.0 to 1.0)
    pub confidence: f32,
    /// Reasoning for the suggested action
    pub reasoning: String,
}

/// Analysis result for a rebase operation
#[derive(Debug, Clone)]
pub struct RebaseAnalysis {
    /// Commits that will be rebased
    pub commits: Vec<RebaseCommit>,
    /// Upstream reference
    pub upstream: String,
    /// Branch being rebased
    pub branch: String,
    /// Total number of operations suggested
    pub suggested_operations: usize,
}

/// Result of a rebase operation
#[derive(Debug, Clone)]
pub struct RebaseResult {
    /// Number of operations performed
    pub operations_performed: usize,
    /// Number of commits processed
    pub commits_processed: usize,
    /// Whether the rebase completed successfully
    pub success: bool,
    /// Any conflicts encountered
    pub conflicts: Vec<String>,
}