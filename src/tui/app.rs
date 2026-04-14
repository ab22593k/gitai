use super::renderer::draw_ui;
use super::runtime::{ExitStatus, TerminalGuard, TuiRuntime};
use super::spinner::SpinnerState;
use super::state::{Mode, TuiState};
use super::task_runner::TuiTaskRunner;
use crate::commands::commit::{
    CommitService, completion::CompletionService, format_commit_result, types::GeneratedMessage,
};
use anyhow::{Error, Result};
use crossterm::event::{Event, EventStream, KeyEvent, KeyEventKind};
use futures::StreamExt;
use std::io;
use std::sync::Arc;
use std::time::Duration;

/// Result of processing a single input event.
#[derive(Debug, PartialEq, Eq)]
pub enum InputResult {
    Continue,
    Exit,
    Commit(String),
}

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

    #[allow(clippy::unused_async)]
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

        // Initialize context for selection (ignore errors, regeneration will fall back to default)
        let _ = app.initialize_context().await;

        app.run_app(theme_mode).await.map_err(Error::from)
    }

    pub async fn run_app(&mut self, theme_mode: crate::common::ThemeMode) -> io::Result<()> {
        // Setup terminal with theme (delegated to TuiRuntime)
        let mut guard = TuiRuntime::setup_with_theme(theme_mode)?;

        // Run main loop
        let result = self.main_loop_impl(&mut guard).await;

        // Cleanup happens automatically via TerminalGuard::Drop
        drop(guard);

        // Handle result
        Self::handle_exit_result(result)
    }

    async fn main_loop_impl(&mut self, guard: &mut TerminalGuard) -> Result<ExitStatus> {
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
            // Render if dirty (delegated to renderer)
            if self.state.is_dirty() {
                guard.terminal_mut().draw(|f| draw_ui(f, &mut self.state))?;
                self.state.set_dirty(false);
            }

            // Spawn tasks if needed (delegated to TuiTaskRunner)
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

            // Wait for events
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

            // 1. Ticker tick
            _ = ticker.tick() => {
                if self.state.mode() == Mode::Generating
                    && let Some(spinner) = self.state.spinner_mut() {
                        spinner.tick();
                        self.state.set_dirty(true);
                    }
                Ok(LoopResult::Continue)
            }

            // 2. Generation result
            Some(result) = generation_rx.recv() => {
                self.handle_generation_result(result);
                Ok(LoopResult::Continue)
            }

            // 3. Completion result
            Some(result) = completion_rx.recv() => {
                self.handle_completion_result(result);
                Ok(LoopResult::Continue)
            }

            // 4. User input
            maybe_event = events.next() => {
                if let Some(Ok(Event::Key(key))) = maybe_event
                    && key.kind == KeyEventKind::Press {
                        let input_result = handle_input_with_state(&mut self.state, key);
                        match input_result {
                            InputResult::Exit => Ok(LoopResult::Exit(ExitStatus::Cancelled)),
                            InputResult::Commit(message) => {
                                let status = self.perform_commit_impl(&message);
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
                ExitStatus::Committed(message) => {
                    println!("{message}");
                }
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

    fn perform_commit_impl(&self, message: &str) -> ExitStatus {
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

/// Handle input given only `TuiState`
fn handle_input_with_state(state: &mut TuiState, key: KeyEvent) -> InputResult {
    match state.mode() {
        Mode::Normal => handle_normal_mode(state, key),
        Mode::EditingMessage => handle_editing_message_mode(state, key),
        Mode::EditingInstructions => handle_editing_instructions_mode(state, key),
        Mode::Generating => InputResult::Continue,
        Mode::Help => handle_help_mode(state, key),
        Mode::Completing => handle_completing_mode(state, key),
        Mode::ContextSelection => handle_context_selection_mode(state, key),
    }
}

fn handle_normal_mode(state: &mut TuiState, key: KeyEvent) -> InputResult {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => InputResult::Exit,
        KeyCode::Enter => {
            let message = format!(
                "{}\n\n{}",
                state.current_message().title,
                state.current_message().message
            );
            InputResult::Commit(message)
        }
        KeyCode::Char('E') => {
            state.set_mode(Mode::EditingMessage);
            state.set_status(
                "Editing commit message... Press 'Esc' to finish, 'Tab' for completion",
            );
            InputResult::Continue
        }
        KeyCode::Char('I') => {
            state.set_mode(Mode::EditingInstructions);
            if !state.is_instructions_visible() {
                state.toggle_instructions_visibility();
            }
            state.set_status("Editing instructions... Press 'Esc' to finish");
            InputResult::Continue
        }
        KeyCode::Char('R') => {
            state.set_mode(Mode::Generating);
            state.set_spinner(Some(SpinnerState::new()));
            state.set_status("Regenerating commit message...");
            state.set_dirty(true);
            InputResult::Continue
        }
        KeyCode::Char('?') => {
            state.set_mode(Mode::Help);
            InputResult::Continue
        }
        KeyCode::Char('C') => {
            state.set_mode(Mode::ContextSelection);
            state.set_status(
                "Select context: 'Space' toggle, 'Tab' switch category, 'Enter' confirm, 'Esc' cancel",
            );
            InputResult::Continue
        }
        KeyCode::Left => {
            state.previous_message();
            state.set_status(format!(
                " Message {}/{}",
                state.current_index() + 1,
                state.messages().len()
            ));
            InputResult::Continue
        }
        KeyCode::Right => {
            state.next_message();
            state.set_status(format!(
                " Message {}/{}",
                state.current_index() + 1,
                state.messages().len()
            ));
            InputResult::Continue
        }
        KeyCode::Up => {
            state.message_textarea_mut().scroll((-1, 0));
            state.set_dirty(true);
            InputResult::Continue
        }
        KeyCode::Down => {
            state.message_textarea_mut().scroll((1, 0));
            state.set_dirty(true);
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}

fn handle_editing_message_mode(state: &mut TuiState, key: KeyEvent) -> InputResult {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Esc => {
            state.set_mode(Mode::Normal);
            state.update_current_message_from_textarea();
            state.set_status(" Edited message saved. Press 'Enter' to commit.");
            InputResult::Continue
        }
        KeyCode::Tab => {
            let (row, col) = state.message_textarea().cursor();
            let lines = state.message_textarea().lines();
            if row < lines.len() {
                let line = &lines[row];
                if col <= line.len() {
                    let prefix = line[..col].to_string();
                    if !prefix.trim().is_empty() {
                        state.set_pending_completion_prefix(Some(prefix));
                        state.set_mode(Mode::Completing);
                        state.set_status("Generating completion suggestions...");
                        state.set_dirty(true);
                    }
                }
            }
            InputResult::Continue
        }
        _ => {
            state.message_textarea_mut().input(key);
            state.set_dirty(true);
            InputResult::Continue
        }
    }
}

fn handle_editing_instructions_mode(state: &mut TuiState, key: KeyEvent) -> InputResult {
    use crossterm::event::KeyCode;
    if key.code == KeyCode::Esc {
        state.set_mode(Mode::Normal);
        state.update_instructions_from_textarea();
        state.set_status(" Instructions updated. Press 'R' to regenerate.");
        InputResult::Continue
    } else {
        state.instructions_textarea_mut().input(key);
        state.set_dirty(true);
        InputResult::Continue
    }
}

fn handle_help_mode(state: &mut TuiState, _key: KeyEvent) -> InputResult {
    state.set_mode(Mode::Normal);
    state.set_status("Press '?': help | 'Esc': exit");
    InputResult::Continue
}

fn handle_completing_mode(state: &mut TuiState, key: KeyEvent) -> InputResult {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Esc => {
            state.set_mode(Mode::EditingMessage);
            state.set_status("Completion cancelled.");
            state.set_completion_suggestions(Vec::new());
            InputResult::Continue
        }
        KeyCode::Enter => {
            if !state.completion_suggestions().is_empty() {
                let suggestion = state.completion_suggestions()[state.completion_index()].clone();
                state.message_textarea_mut().insert_str(&suggestion);
                state.set_completion_suggestions(Vec::new());
                state.set_mode(Mode::EditingMessage);
                state.set_status(" Completion applied.");
            }
            InputResult::Continue
        }
        KeyCode::Tab | KeyCode::Down => {
            state.next_completion();
            InputResult::Continue
        }
        KeyCode::BackTab | KeyCode::Up => {
            state.previous_completion();
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}

fn handle_context_selection_mode(state: &mut TuiState, key: KeyEvent) -> InputResult {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Esc => {
            state.set_mode(Mode::Normal);
            state.set_status("Context selection cancelled.");
            InputResult::Continue
        }
        KeyCode::Enter => {
            state.set_mode(Mode::Normal);
            state.set_status(" Context updated. Press 'R' to regenerate with new context.");
            InputResult::Continue
        }
        KeyCode::Up => {
            state.move_selection_up();
            InputResult::Continue
        }
        KeyCode::Down => {
            state.move_selection_down();
            InputResult::Continue
        }
        KeyCode::Tab => {
            state.next_category();
            InputResult::Continue
        }
        KeyCode::Char(' ') => {
            state.toggle_current_selection();
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}

#[allow(clippy::unused_async)]
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
        assert_eq!(state.messages()[0].title, "Initial commit");

        let new_message = GeneratedMessage {
            title: "Regenerated commit".to_string(),
            message: "Regenerated message".to_string(),
        };

        state.add_message(new_message);

        assert_eq!(state.messages().len(), 3, "Should add a new message");
        assert_eq!(
            state.current_index(),
            2,
            "Current index should point to new message"
        );
        assert_eq!(
            state.messages()[2].title,
            "Regenerated commit",
            "New message should be added"
        );
        assert_eq!(
            state.messages()[0].title,
            "Initial commit",
            "Original messages should be unchanged"
        );
        assert_eq!(
            state.messages()[1].title,
            "Second commit",
            "Other messages should be unchanged"
        );
    }

    #[test]
    fn test_regeneration_with_empty_messages() {
        let initial_messages = vec![];
        let mut state = TuiState::new(initial_messages, "test instructions".to_string());

        assert_eq!(state.messages().len(), 1);
        assert_eq!(state.current_index(), 0);

        let new_message = GeneratedMessage {
            title: "New commit".to_string(),
            message: "New message".to_string(),
        };

        state.add_message(new_message);

        assert_eq!(state.messages().len(), 2, "Should add new message");
        assert_eq!(state.current_index(), 1, "Should switch to new message");
        assert_eq!(state.messages()[1].title, "New commit");
    }

    #[test]
    fn test_regeneration_always_adds_message() {
        let initial_messages = vec![GeneratedMessage {
            title: "First commit".to_string(),
            message: "First message".to_string(),
        }];

        let mut state = TuiState::new(initial_messages, "test instructions".to_string());
        assert_eq!(state.messages().len(), 1);
        assert_eq!(state.current_index(), 0);

        let new_message = GeneratedMessage {
            title: "New commit".to_string(),
            message: "New message".to_string(),
        };

        state.add_message(new_message);

        assert_eq!(state.messages().len(), 2);
        assert_eq!(state.current_index(), 1);
        assert_eq!(state.messages()[1].title, "New commit");
        assert_eq!(
            state.messages()[0].title,
            "First commit",
            "Original message should remain"
        );
    }
}
