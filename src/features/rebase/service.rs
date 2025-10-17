//! Rebase service implementation

use super::types::{RebaseAction, RebaseAnalysis, RebaseCommit, RebaseResult};
use crate::config::Config;
use crate::debug;
use crate::git::GitRepo;
use crate::ui;

use anyhow::Result;
use std::sync::Arc;

/// Service for handling AI-assisted rebase operations
pub struct RebaseService {
    config: Config,
    repo: Arc<GitRepo>,
}

impl RebaseService {
    /// Create a new RebaseService instance
    pub fn new(config: Config, repo: GitRepo) -> Result<Self> {
        Ok(Self {
            config,
            repo: Arc::new(repo),
        })
    }

    /// Check if the environment is suitable for rebase operations
    pub fn check_environment(&self) -> Result<()> {
        // Check if we're in a git repository
        if self.repo.is_remote() {
            return Err(anyhow::anyhow!("Cannot perform rebase operations on remote repositories"));
        }

        // Check if there are any uncommitted changes
        let repo_binding = self.repo.open_repo()?;
        let status = repo_binding.statuses(None)?;
        if status.iter().any(|s| s.status() != git2::Status::CURRENT) {
            ui::print_warning("You have uncommitted changes. Consider committing or stashing them before rebasing.");
        }

        Ok(())
    }

    /// Analyze commits that will be rebased and suggest actions
    pub async fn analyze_commits_for_rebase(
        &self,
        upstream: &str,
        branch: Option<&str>,
    ) -> Result<RebaseAnalysis> {
        debug!("Analyzing commits for rebase: upstream={}, branch={:?}", upstream, branch);

        let repo = self.repo.open_repo()?;

        // Determine which branch to rebase
        let default_branch = self.repo.get_current_branch().unwrap_or("HEAD".to_string());
        let branch_name = branch.unwrap_or(default_branch.as_str());

        // Find the merge base between upstream and branch
        let upstream_commit = repo.revparse_single(upstream)?.peel_to_commit()?;
        let branch_commit = repo.revparse_single(&branch_name)?.peel_to_commit()?;

        let merge_base_oid = repo.merge_base(upstream_commit.id(), branch_commit.id())?;
        let merge_base = repo.find_commit(merge_base_oid)?;

        // Get all commits from merge_base to branch_commit
        let mut revwalk = repo.revwalk()?;
        revwalk.push(branch_commit.id())?;
        revwalk.hide(merge_base.id())?;

        let mut commits = Vec::new();
        for oid_result in revwalk {
            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;

            let rebase_commit = RebaseCommit {
                hash: format!("{:?}", oid).chars().take(7).collect(),
                message: commit.message().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("Unknown").to_string(),
                date: format!("{}", commit.time().seconds()), // TODO: Format properly
                suggested_action: RebaseAction::Pick, // Default to pick, will be analyzed
                confidence: 0.5,
                reasoning: "Default action".to_string(),
            };

            commits.push(rebase_commit);
        }

        // Reverse to get chronological order (oldest first)
        commits.reverse();

        // Analyze commits with AI to suggest actions
        let analyzed_commits = self.analyze_commit_actions(commits).await?;

        let analysis = RebaseAnalysis {
            commits: analyzed_commits,
            upstream: upstream.to_string(),
            branch: branch_name.to_string(),
            suggested_operations: 0, // TODO: Calculate based on non-pick actions
        };

        Ok(analysis)
    }

    /// Analyze commits and suggest rebase actions using AI
    async fn analyze_commit_actions(&self, commits: Vec<RebaseCommit>) -> Result<Vec<RebaseCommit>> {
        // For now, just return the commits with default actions
        // TODO: Implement AI analysis for suggesting actions
        Ok(commits.into_iter().map(|mut commit| {
            // Simple heuristics for now
            if commit.message.to_lowercase().contains("fix") {
                commit.suggested_action = RebaseAction::Pick;
                commit.reasoning = "Fix commits are typically kept as-is".to_string();
            } else if commit.message.to_lowercase().contains("wip") {
                commit.suggested_action = RebaseAction::Squash;
                commit.reasoning = "WIP commits should be squashed".to_string();
            } else {
                commit.suggested_action = RebaseAction::Pick;
                commit.reasoning = "Standard commit, keep as-is".to_string();
            }
            commit.confidence = 0.7;
            commit
        }).collect())
    }

    /// Perform rebase with auto-applied AI suggestions
    pub async fn perform_rebase_auto(&self, analysis: RebaseAnalysis) -> Result<RebaseResult> {
        debug!("Performing auto rebase with {} commits", analysis.commits.len());

        // For now, just simulate the rebase
        ui::print_info("Simulating rebase operations...");

        let mut operations = 0;
        for commit in &analysis.commits {
            match commit.suggested_action {
                RebaseAction::Pick => {
                    ui::print_info(&format!("Picking: {}", commit.message.lines().next().unwrap_or("")));
                }
                RebaseAction::Reword => {
                    ui::print_info(&format!("Rewording: {}", commit.message.lines().next().unwrap_or("")));
                    operations += 1;
                }
                RebaseAction::Squash => {
                    ui::print_info(&format!("Squashing: {}", commit.message.lines().next().unwrap_or("")));
                    operations += 1;
                }
                RebaseAction::Fixup => {
                    ui::print_info(&format!("Fixup: {}", commit.message.lines().next().unwrap_or("")));
                    operations += 1;
                }
                RebaseAction::Drop => {
                    ui::print_info(&format!("Dropping: {}", commit.message.lines().next().unwrap_or("")));
                    operations += 1;
                }
                RebaseAction::Edit => {
                    ui::print_info(&format!("Editing: {}", commit.message.lines().next().unwrap_or("")));
                    operations += 1;
                }
            }
        }

        // TODO: Actually perform the rebase operations

        Ok(RebaseResult {
            operations_performed: operations,
            commits_processed: analysis.commits.len(),
            success: true,
            conflicts: vec![],
        })
    }
}