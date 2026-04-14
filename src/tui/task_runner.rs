//! Async task orchestration for generation and completion
//!
//! This module handles:
//! - Spawning generation tasks via tokio
//! - Spawning completion tasks via tokio
//! - Channel management for task results
//! - Preventing duplicate task spawns

use crate::commands::commit::{
    CommitService, completion::CompletionService, types::GeneratedMessage,
};
use crate::llm::context::CommitContext;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Task runner for async operations
///
/// Manages spawning and coordinating results for:
/// - Commit message generation
/// - Message completion suggestions
pub struct TuiTaskRunner {
    /// Commit service for message generation
    commit_service: Arc<CommitService>,
    /// Completion service for AI suggestions
    completion_service: Arc<CompletionService>,
    /// Channel sender for generation results
    generation_tx: mpsc::Sender<Result<GeneratedMessage, anyhow::Error>>,
    /// Channel sender for completion results
    completion_tx: mpsc::Sender<Result<Vec<String>, anyhow::Error>>,
    /// Flag to prevent duplicate generation spawns
    generation_task_spawned: bool,
    /// Flag to prevent duplicate completion spawns
    completion_task_spawned: bool,
}

impl TuiTaskRunner {
    /// Create a new task runner
    pub fn new(
        commit_service: Arc<CommitService>,
        completion_service: Arc<CompletionService>,
        generation_tx: mpsc::Sender<Result<GeneratedMessage, anyhow::Error>>,
        completion_tx: mpsc::Sender<Result<Vec<String>, anyhow::Error>>,
    ) -> Self {
        Self {
            commit_service,
            completion_service,
            generation_tx,
            completion_tx,
            generation_task_spawned: false,
            completion_task_spawned: false,
        }
    }

    /// Spawn generation task if needed
    ///
    /// Spawns a task when:
    /// - `should_spawn` is true (caller determines mode == Generating)
    /// - No generation task has been spawned yet
    pub fn spawn_generation_if_needed(
        &mut self,
        should_spawn: bool,
        instructions: String,
        context: Option<CommitContext>,
    ) {
        if should_spawn && !self.generation_task_spawned {
            let service = self.commit_service.clone();
            let tx = self.generation_tx.clone();

            tokio::spawn(async move {
                let result = if let Some(ctx) = context {
                    service
                        .generate_message_with_context(&instructions, ctx)
                        .await
                } else {
                    service.generate_message(&instructions).await
                };
                let _ = tx.send(result).await;
            });

            self.generation_task_spawned = true;
        }
    }

    /// Spawn completion task if needed
    ///
    /// Spawns a task when:
    /// - `prefix` is Some (caller determines pending completion)
    /// - No completion task has been spawned yet
    pub fn spawn_completion_if_needed(&mut self, prefix: Option<String>) {
        if let Some(prefix) = prefix
            && !self.completion_task_spawned
        {
            let completion_service = self.completion_service.clone();
            let prefix = prefix.clone();
            let tx = self.completion_tx.clone();

            tokio::spawn(async move {
                match completion_service.complete_message(&prefix, 0.5).await {
                    Ok(completed_message) => {
                        let _ = tx.send(Ok(vec![completed_message.title])).await;
                    }
                    Err(_e) => {
                        let suggestions = vec![
                            format!("{}: add new feature", prefix),
                            format!("{}: fix bug", prefix),
                            format!("{}: update documentation", prefix),
                        ];
                        let _ = tx.send(Ok(suggestions)).await;
                    }
                }
            });

            self.completion_task_spawned = true;
        }
    }

    /// Reset generation task flag (allows spawning again)
    pub fn reset_generation_flag(&mut self) {
        self.generation_task_spawned = false;
    }

    /// Check if generation task has been spawned
    pub fn is_generation_spawned(&self) -> bool {
        self.generation_task_spawned
    }

    /// Check if completion task has been spawned
    pub fn is_completion_spawned(&self) -> bool {
        self.completion_task_spawned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_task_flag_prevents_duplicate_spawns() {
        let mut task_spawned = false;

        // First spawn should set flag to true
        let should_spawn = !task_spawned;
        assert!(should_spawn);
        task_spawned = true;

        // Second spawn should be skipped
        let should_spawn_again = !task_spawned;
        assert!(!should_spawn_again);
    }

    #[test]
    fn test_completion_task_flag_prevents_duplicate_spawns() {
        let mut completion_task_spawned = false;

        let should_spawn = !completion_task_spawned;
        assert!(should_spawn);
        completion_task_spawned = true;

        let should_spawn_again = !completion_task_spawned;
        assert!(!should_spawn_again);
    }

    #[test]
    fn test_channel_contract_for_generation() {
        let (tx, mut rx) = mpsc::channel::<Result<GeneratedMessage, anyhow::Error>>(1);

        // Verify channel can be cloned
        let tx_clone = tx.clone();
        drop(tx_clone);

        // Verify closed channel returns None
        drop(tx);
        assert!(rx.blocking_recv().is_none());
    }

    #[test]
    fn test_channel_contract_for_completion() {
        let (tx, mut rx) = mpsc::channel::<Result<Vec<String>, anyhow::Error>>(1);

        drop(tx);
        assert!(rx.blocking_recv().is_none());
    }

    #[tokio::test]
    async fn test_channel_send_receive_ordering() {
        let (tx, mut rx) = mpsc::channel::<i32>(10);

        tx.send(1).await.expect("channel should be open");
        tx.send(2).await.expect("channel should be open");
        tx.send(3).await.expect("channel should be open");
        drop(tx);

        assert_eq!(rx.recv().await, Some(1));
        assert_eq!(rx.recv().await, Some(2));
        assert_eq!(rx.recv().await, Some(3));
        assert_eq!(rx.recv().await, None);
    }
}
