use super::input_handler::{InputResult, handle_input};
use super::spinner::SpinnerState;
use super::state::{Mode, TuiState};
use super::theme::init_theme;
use super::ui::draw_ui;
use crate::features::commit::{
    CommitService, completion::CompletionService, format_commit_result, types::GeneratedMessage,
};
use anyhow::{Error, Result};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        event::{self, Event},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};

use log::debug;
use std::io;
use std::panic;
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
        // Initialize adaptive theme
        init_theme(theme_mode);

        // Setup
        let default_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info: &panic::PanicHookInfo| {
            let _ = crossterm::terminal::disable_raw_mode();
            default_hook(panic_info);
        }));
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run main loop
        let result = self.main_loop(&mut terminal).await;

        // Cleanup
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        // Handle result and display appropriate message
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

    #[allow(clippy::too_many_lines)]
    async fn main_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> anyhow::Result<ExitStatus> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<GeneratedMessage, anyhow::Error>>(1);
        let (completion_tx, mut completion_rx) =
            tokio::sync::mpsc::channel::<Result<Vec<String>, anyhow::Error>>(1);
        let mut task_spawned = false;
        let mut completion_task_spawned = false;

        loop {
            // Redraw only if dirty
            if self.state.dirty {
                terminal.draw(|f| draw_ui(f, &mut self.state))?;
                self.state.dirty = false; // Reset dirty flag after redraw
            }

            // Spawn the task only once when entering the Generating mode
            if self.state.mode == Mode::Generating && !task_spawned {
                let service = self.service.clone();
                let instructions = self.state.custom_instructions.clone();
                let filtered_context = self.state.get_filtered_context();
                let tx = tx.clone();

                tokio::spawn(async move {
                    // Use filtered context if available, otherwise use default
                    let result = if let Some(context) = filtered_context {
                        service
                            .generate_message_with_context(&instructions, context)
                            .await
                    } else {
                        service.generate_message(&instructions).await
                    };
                    let _ = tx.send(result).await;
                });

                task_spawned = true; // Ensure we only spawn the task once
            }

            // Spawn completion task if there's a pending completion request
            if let Some(prefix) = &self.state.pending_completion_prefix.clone()
                && !completion_task_spawned
            {
                let completion_service = self.completion_service.clone();
                let prefix = prefix.clone();
                let completion_tx = completion_tx.clone();

                tokio::spawn(async move {
                    debug!("Generating completion for prefix: {prefix}");
                    // Generate real completion suggestions using the completion service
                    match completion_service.complete_message(&prefix, 0.5).await {
                        Ok(completed_message) => {
                            // Extract the completed title as a suggestion
                            let suggestion = completed_message.title;
                            let suggestions = vec![suggestion];
                            let _ = completion_tx.send(Ok(suggestions)).await;
                        }
                        Err(e) => {
                            debug!("Completion failed: {e}");
                            // Fallback to basic suggestions if completion fails
                            let suggestions = vec![
                                format!("{}: add new feature", prefix),
                                format!("{}: fix bug", prefix),
                                format!("{}: update documentation", prefix),
                            ];
                            let _ = completion_tx.send(Ok(suggestions)).await;
                        }
                    }
                });

                completion_task_spawned = true;
                self.state.pending_completion_prefix = None; // Clear the pending request
            }

            // Check if a message has been received from the generation task
            match rx.try_recv() {
                Ok(result) => match result {
                    Ok(new_message) => {
                        // Add the new message to the list and switch to it
                        self.state.messages.push(new_message);
                        self.state.current_index = self.state.messages.len() - 1;

                        self.state.update_message_textarea();
                        self.state.mode = Mode::Normal; // Exit Generating mode
                        self.state.spinner = None; // Stop the spinner
                        self.state.set_status(format!(
                            "New message generated! Viewing {}/{}",
                            self.state.current_index + 1,
                            self.state.messages.len()
                        ));
                        task_spawned = false; // Reset for future regenerations
                    }
                    Err(e) => {
                        self.state.mode = Mode::Normal; // Exit Generating mode
                        self.state.spinner = None; // Stop the spinner
                        self.state.set_status(format!(
                            "Generation failed: {e}. Press 'R' to retry or 'Esc' to exit."
                        ));
                        task_spawned = false; // Reset for future regenerations
                    }
                },
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    // No message available yet, continue the loop
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    // Handle the case where the sender has disconnected
                    task_spawned = false;
                }
            }

            // Check if completion suggestions have been received
            match completion_rx.try_recv() {
                Ok(result) => match result {
                    Ok(suggestions) => {
                        self.state.completion_suggestions = suggestions;
                        self.state.completion_index = 0;
                        completion_task_spawned = false;
                    }
                    Err(e) => {
                        self.state.set_status(format!(
                            "Completion failed: {e}. Press Tab to retry or continue editing."
                        ));
                        self.state.mode = Mode::EditingMessage;
                        completion_task_spawned = false;
                    }
                },
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    // No completion available yet, continue the loop
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    // Handle the case where the sender has disconnected
                    break;
                }
            }

            // Poll for input events asynchronously
            let event = tokio::task::spawn_blocking(|| {
                if event::poll(Duration::from_millis(20)).unwrap_or(false) {
                    if let Ok(Event::Key(key)) = event::read() {
                        if key.kind == crossterm::event::KeyEventKind::Press {
                            Some(key)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .await
            .expect("Failed to poll for input events");

            if let Some(key) = event {
                let input_result = handle_input(self, key).await;
                match input_result {
                    InputResult::Exit => return Ok(ExitStatus::Cancelled),
                    InputResult::Commit(message) => match self.perform_commit(&message) {
                        Ok(status) => return Ok(status),
                        Err(e) => {
                            self.state.set_status(format!(
                                "Commit failed: {e}. Check your staged changes and try again."
                            ));
                            self.state.dirty = true;
                        }
                    },
                    InputResult::Continue => self.state.dirty = true,
                }
            }

            // Update the spinner state and redraw if in generating mode
            if self.state.mode == Mode::Generating
                && self.state.last_spinner_update.elapsed() >= Duration::from_millis(100)
            {
                if let Some(spinner) = &mut self.state.spinner {
                    spinner.tick();
                    self.state.dirty = true; // Mark dirty to trigger redraw
                }
                self.state.last_spinner_update = std::time::Instant::now(); // Reset the update time
            }
        }

        Ok(ExitStatus::Cancelled)
    }

    pub fn handle_regenerate(&mut self) {
        self.state.mode = Mode::Generating;
        self.state.spinner = Some(SpinnerState::new());
        self.state
            .set_status(String::from("Regenerating commit message..."));
        self.state.dirty = true; // Make sure UI updates
    }

    pub fn perform_commit(&self, message: &str) -> Result<ExitStatus, Error> {
        match self.service.perform_commit(message, false, None) {
            Ok(result) => {
                let output = format_commit_result(&result, message);
                Ok(ExitStatus::Committed(output))
            }
            Err(e) => Ok(ExitStatus::Error(e.to_string())),
        }
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

pub enum ExitStatus {
    Committed(String),
    Cancelled,
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::commit::types::GeneratedMessage;

    #[test]
    fn test_panic_hook_setup() {
        // Test that the panic hook code compiles and the closure is valid
        // Note: Actual panic hook testing is challenging due to global state
        #[allow(unused_variables)]
        #[allow(clippy::no_effect_underscore_binding)]
        let _closure = |panic_info: &panic::PanicHookInfo| {
            let _ = crossterm::terminal::disable_raw_mode();
        };
        // If this compiles, the setup is correct
    }

    #[test]
    fn test_regeneration_adds_new_message() {
        // Test that regeneration adds a new message and switches to it
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
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.current_index, 0);
        assert_eq!(state.messages[0].title, "Initial commit");

        // Simulate regeneration result: add new message
        let new_message = GeneratedMessage {
            title: "Regenerated commit".to_string(),
            message: "Regenerated message".to_string(),
        };

        // This simulates the logic in the main loop when regeneration succeeds
        state.messages.push(new_message);
        state.current_index = state.messages.len() - 1;

        // Verify the message was added and we're viewing it
        assert_eq!(state.messages.len(), 3, "Should add a new message");
        assert_eq!(
            state.current_index, 2,
            "Current index should point to new message"
        );
        assert_eq!(
            state.messages[2].title, "Regenerated commit",
            "New message should be added"
        );
        assert_eq!(
            state.messages[0].title, "Initial commit",
            "Original messages should be unchanged"
        );
        assert_eq!(
            state.messages[1].title, "Second commit",
            "Other messages should be unchanged"
        );
    }

    #[test]
    fn test_regeneration_with_empty_messages() {
        // Test regeneration when messages vector is empty (edge case)
        let initial_messages = vec![];
        let mut state = TuiState::new(initial_messages, "test instructions".to_string());

        // TuiState::new should create a default message when initial_messages is empty
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.current_index, 0);

        // Simulate regeneration result
        let new_message = GeneratedMessage {
            title: "New commit".to_string(),
            message: "New message".to_string(),
        };

        // This simulates the logic in the main loop
        state.messages.push(new_message);
        state.current_index = state.messages.len() - 1;

        // Verify the message was added
        assert_eq!(state.messages.len(), 2, "Should add new message");
        assert_eq!(state.current_index, 1, "Should switch to new message");
        assert_eq!(state.messages[1].title, "New commit");
    }

    #[test]
    fn test_regeneration_always_adds_message() {
        // Test that regeneration always adds a new message regardless of current_index
        let initial_messages = vec![GeneratedMessage {
            title: "First commit".to_string(),
            message: "First message".to_string(),
        }];

        let mut state = TuiState::new(initial_messages, "test instructions".to_string());
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.current_index, 0);

        let new_message = GeneratedMessage {
            title: "New commit".to_string(),
            message: "New message".to_string(),
        };

        // This simulates the logic in the main loop - always add new message
        state.messages.push(new_message);
        state.current_index = state.messages.len() - 1;

        // Should add the message
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.current_index, 1);
        assert_eq!(state.messages[1].title, "New commit");
        assert_eq!(
            state.messages[0].title, "First commit",
            "Original message should remain"
        );
    }
}
