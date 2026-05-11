//! TUI event loop coordinator.
//!
//! Orchestrates the main loop: rendering, task spawning, event multiplexing.

use super::input::{InputResult, handle_input};
use super::renderer::draw_ui;
use super::runtime::{ExitStatus, TerminalGuard, TuiRuntime};
use super::spinner::SpinnerState;
use super::state::{Mode, TuiState};
use super::task_runner::TuiTaskRunner;
use crate::commands::commit::{
    CommitService, completion::CompletionService, format_commit_result, types::GeneratedMessage,
};
use anyhow::{Error, Result};
use crossterm::event::{EventStream, KeyEventKind};
use futures::StreamExt;
use std::io;
use std::sync::Arc;
use std::time::Duration;

pub struct TuiCommit {
    pub state: TuiState,
    service: Arc<CommitService>,
    completion_service: Arc<CompletionService>,
}

impl TuiCommit {
    pub fn new(
        initial_messages: Vec<GeneratedMessage>,
        custom_instructions: String,
        service: Arc<CommitService>,
        completion_service: Arc<CompletionService>,
    ) -> Self {
        let state = TuiState::new(initial_messages, custom_instructions);
        Self {
            state,
            service,
            completion_service,
        }
    }

    /// Initialize context for selection (call this after creation)
    pub async fn initialize_context(&mut self) -> Result<(), anyhow::Error> {
        let context = self.service.get_git_info().await?;
        self.state.initialize_context(context);
        Ok(())
    }

    pub async fn run(
        initial_messages: Vec<GeneratedMessage>,
        custom_instructions: String,
        service: Arc<CommitService>,
        completion_service: Arc<CompletionService>,
        theme_mode: crate::common::ThemeMode,
    ) -> Result<()> {
        let mut app = Self::new(
            initial_messages,
            custom_instructions,
            service,
            completion_service,
        );
        if let Err(e) = app.initialize_context().await {
            log::warn!("Context initialization failed: {e}");
        }
        app.run_app(theme_mode).await.map_err(Error::from)
    }

    pub async fn run_app(&mut self, theme_mode: crate::common::ThemeMode) -> io::Result<()> {
        let mut guard = TuiRuntime::setup_with_theme(theme_mode)?;
        let result = self.main_loop(&mut guard).await;
        drop(guard);
        Self::handle_exit_result(result)
    }

    async fn main_loop(&mut self, guard: &mut TerminalGuard) -> Result<ExitStatus> {
        let (generation_tx, mut generation_rx) =
            tokio::sync::mpsc::channel::<Result<GeneratedMessage, anyhow::Error>>(1);
        let (completion_tx, mut completion_rx) =
            tokio::sync::mpsc::channel::<Result<Vec<String>, anyhow::Error>>(1);

        let mut task_runner = TuiTaskRunner::new(
            self.service.clone(),
            self.completion_service.clone(),
            generation_tx,
            completion_tx,
        );

        let mut events = EventStream::new();
        let mut ticker = tokio::time::interval(Duration::from_millis(100));

        loop {
            if self.state.is_dirty() {
                guard.terminal_mut().draw(|f| draw_ui(f, &mut self.state))?;
                self.state.set_dirty(false);
            }

            if self.state.mode() == Mode::Generating && !task_runner.is_generation_spawned() {
                let instructions = self.state.custom_instructions().to_string();
                let filtered_context = self.state.get_filtered_context();
                task_runner.spawn_generation_if_needed(true, instructions, filtered_context);
            }
            if self.state.mode() != Mode::Generating && task_runner.is_generation_spawned() {
                task_runner.reset_generation_flag();
            }

            if let Some(prefix) = self.state.pending_completion_prefix().cloned()
                && !task_runner.is_completion_spawned()
            {
                task_runner.spawn_completion_if_needed(Some(prefix));
                self.state.set_pending_completion_prefix(None);
            }

            match self
                .wait_for_events(
                    &mut generation_rx,
                    &mut completion_rx,
                    &mut events,
                    &mut ticker,
                )
                .await?
            {
                LoopResult::Continue => {}
                LoopResult::Exit(status) => return Ok(status),
            }
        }
    }

    async fn wait_for_events(
        &mut self,
        generation_rx: &mut tokio::sync::mpsc::Receiver<Result<GeneratedMessage, anyhow::Error>>,
        completion_rx: &mut tokio::sync::mpsc::Receiver<Result<Vec<String>, anyhow::Error>>,
        events: &mut EventStream,
        ticker: &mut tokio::time::Interval,
    ) -> Result<LoopResult> {
        tokio::select! {
            biased;

            _ = ticker.tick() => {
                if self.state.mode() == Mode::Generating
                    && let Some(spinner) = self.state.spinner_mut() {
                        spinner.tick();
                        self.state.set_dirty(true);
                    }
                Ok(LoopResult::Continue)
            }

            Some(result) = generation_rx.recv() => {
                self.handle_generation_result(result);
                Ok(LoopResult::Continue)
            }

            Some(result) = completion_rx.recv() => {
                self.handle_completion_result(result);
                Ok(LoopResult::Continue)
            }

            maybe_event = events.next() => {
                if let Some(Ok(crossterm::event::Event::Key(key))) = maybe_event
                    && key.kind == KeyEventKind::Press {
                        let input_result = handle_input(&mut self.state, key);
                        match input_result {
                            InputResult::Exit => Ok(LoopResult::Exit(ExitStatus::Cancelled)),
                            InputResult::Commit(message) => {
                                let status = self.perform_commit(&message);
                                Ok(LoopResult::Exit(status))
                            },
                            InputResult::Continue => {
                                self.state.set_dirty(true);
                                Ok(LoopResult::Continue)
                            }
                        }
                    } else {
                        Ok(LoopResult::Continue)
                    }
            }
        }
    }

    fn handle_generation_result(&mut self, result: Result<GeneratedMessage, anyhow::Error>) {
        match result {
            Ok(new_message) => {
                self.state.add_message(new_message);
                self.state.set_mode(Mode::Normal);
                self.state.set_spinner(None);
                self.state.set_status(format!(
                    "New message generated! Viewing {}/{}",
                    self.state.current_index() + 1,
                    self.state.messages().len()
                ));
            }
            Err(e) => {
                self.state.set_mode(Mode::Normal);
                self.state.set_spinner(None);
                self.state.set_status(format!(
                    "Generation failed: {e}. Press 'R' to retry or 'Esc' to exit."
                ));
            }
        }
    }

    fn handle_completion_result(&mut self, result: Result<Vec<String>, anyhow::Error>) {
        match result {
            Ok(suggestions) => {
                self.state.set_completion_suggestions(suggestions);
            }
            Err(e) => {
                self.state.set_status(format!(
                    "Completion failed: {e}. Press Tab to retry or continue editing."
                ));
                self.state.set_mode(Mode::EditingMessage);
            }
        }
    }

    fn handle_exit_result(result: Result<ExitStatus>) -> io::Result<()> {
        match result {
            Ok(exit_status) => match exit_status {
                ExitStatus::Committed(message) => println!("{message}"),
                ExitStatus::Cancelled => {
                    println!("Commit operation cancelled. Your changes remain staged.");
                }
                ExitStatus::Error(error_message) => {
                    eprintln!("An error occurred: {error_message}");
                }
            },
            Err(e) => {
                eprintln!("An unexpected error occurred: {e}");
                return Err(io::Error::other(e.to_string()));
            }
        }
        Ok(())
    }

    fn perform_commit(&self, message: &str) -> ExitStatus {
        match self.service.perform_commit(message, false, None) {
            Ok(result) => {
                let output = format_commit_result(&result, message);
                ExitStatus::Committed(output)
            }
            Err(e) => ExitStatus::Error(e.to_string()),
        }
    }

    pub fn handle_regenerate(&mut self) {
        self.state.set_mode(Mode::Generating);
        self.state.set_spinner(Some(SpinnerState::new()));
        self.state
            .set_status(String::from("Regenerating commit message..."));
        self.state.set_dirty(true);
    }
}

enum LoopResult {
    Continue,
    Exit(ExitStatus),
}

pub async fn run_tui_commit(
    initial_messages: Vec<GeneratedMessage>,
    custom_instructions: String,
    service: Arc<CommitService>,
    completion_service: Arc<CompletionService>,
    theme_mode: crate::common::ThemeMode,
) -> Result<()> {
    TuiCommit::run(
        initial_messages,
        custom_instructions,
        service,
        completion_service,
        theme_mode,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::commit::types::GeneratedMessage;

    #[test]
    fn test_regeneration_adds_new_message() {
        let initial_messages = vec![
            GeneratedMessage {
                title: "Initial commit".to_string(),
                message: "Initial message".to_string(),
            },
            GeneratedMessage {
                title: "Second commit".to_string(),
                message: "Second message".to_string(),
            },
        ];
        let mut state = TuiState::new(initial_messages, "test instructions".to_string());
        assert_eq!(state.messages().len(), 2);
        assert_eq!(state.current_index(), 0);

        state.add_message(GeneratedMessage {
            title: "Regenerated commit".to_string(),
            message: "Regenerated message".to_string(),
        });

        assert_eq!(state.messages().len(), 3);
        assert_eq!(state.current_index(), 2);
        assert_eq!(state.messages()[2].title, "Regenerated commit");
        assert_eq!(state.messages()[0].title, "Initial commit");
        assert_eq!(state.messages()[1].title, "Second commit");
    }

    #[test]
    fn test_regeneration_with_empty_messages() {
        let mut state = TuiState::new(vec![], "test instructions".to_string());
        assert_eq!(state.messages().len(), 1);
        assert_eq!(state.current_index(), 0);

        state.add_message(GeneratedMessage {
            title: "New commit".to_string(),
            message: "New message".to_string(),
        });

        assert_eq!(state.messages().len(), 2);
        assert_eq!(state.current_index(), 1);
        assert_eq!(state.messages()[1].title, "New commit");
    }

    #[test]
    fn test_regeneration_always_adds_message() {
        let mut state = TuiState::new(
            vec![GeneratedMessage {
                title: "First commit".to_string(),
                message: "First message".to_string(),
            }],
            "test instructions".to_string(),
        );
        assert_eq!(state.messages().len(), 1);

        state.add_message(GeneratedMessage {
            title: "New commit".to_string(),
            message: "New message".to_string(),
        });

        assert_eq!(state.messages().len(), 2);
        assert_eq!(state.current_index(), 1);
        assert_eq!(state.messages()[1].title, "New commit");
        assert_eq!(state.messages()[0].title, "First commit");
    }
}
