use futures::StreamExt;
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
        event::{Event, EventStream},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};

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

        let mut events = EventStream::new();
        let mut ticker = tokio::time::interval(Duration::from_millis(100));

        loop {
            // Redraw only if dirty
            if self.state.is_dirty() {
                terminal.draw(|f| draw_ui(f, &mut self.state))?;
                self.state.set_dirty(false); // Reset dirty flag after redraw
            }

            // Spawn the task only once when entering the Generating mode
            if self.state.mode() == Mode::Generating && !task_spawned {
                let service = self.service.clone();
                let instructions = self.state.custom_instructions().to_string();
                let filtered_context = self.state.get_filtered_context();
                let tx = tx.clone();

                tokio::spawn(async move {
                    let result = if let Some(context) = filtered_context {
                        service
                            .generate_message_with_context(&instructions, context)
                            .await
                    } else {
                        service.generate_message(&instructions).await
                    };
                    let _ = tx.send(result).await;
                });

                task_spawned = true;
            }

            // Spawn completion task if there's a pending completion request
            if let Some(prefix) = self.state.pending_completion_prefix().cloned()
                && !completion_task_spawned
            {
                let completion_service = self.completion_service.clone();
                let prefix = prefix.clone();
                let completion_tx = completion_tx.clone();

                tokio::spawn(async move {
                    match completion_service.complete_message(&prefix, 0.5).await {
                        Ok(completed_message) => {
                            let _ = completion_tx.send(Ok(vec![completed_message.title])).await;
                        }
                        Err(_e) => {
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
                self.state.set_pending_completion_prefix(None);
            }

            tokio::select! {
                _ = ticker.tick() => {
                    if self.state.mode() == Mode::Generating {
                        if let Some(spinner) = self.state.spinner_mut() {
                            spinner.tick();
                            self.state.set_dirty(true);
                        }
                    }
                }
                Some(result) = rx.recv() => {
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
                    task_spawned = false;
                }
                Some(result) = completion_rx.recv() => {
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
                    completion_task_spawned = false;
                }
                maybe_event = events.next() => {
                    if let Some(Ok(Event::Key(key))) = maybe_event {
                        if key.kind == crossterm::event::KeyEventKind::Press {
                            let input_result = handle_input(self, key).await;
                            match input_result {
                                InputResult::Exit => return Ok(ExitStatus::Cancelled),
                                InputResult::Commit(message) => match self.perform_commit(&message) {
                                    Ok(status) => return Ok(status),
                                    Err(e) => {
                                        self.state.set_status(format!(
                                            "Commit failed: {e}. Check your staged changes and try again."
                                        ));
                                        self.state.set_dirty(true);
                                    }
                                },
                                InputResult::Continue => self.state.set_dirty(true),
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn handle_regenerate(&mut self) {
        self.state.set_mode(Mode::Generating);
        self.state.set_spinner(Some(SpinnerState::new()));
        self.state
            .set_status(String::from("Regenerating commit message..."));
        self.state.set_dirty(true); // Make sure UI updates
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
        assert_eq!(state.messages().len(), 2);
        assert_eq!(state.current_index(), 0);
        assert_eq!(state.messages()[0].title, "Initial commit");

        // Simulate regeneration result: add new message
        let new_message = GeneratedMessage {
            title: "Regenerated commit".to_string(),
            message: "Regenerated message".to_string(),
        };

        // This simulates the logic in the main loop when regeneration succeeds
        state.add_message(new_message);

        // Verify the message was added and we're viewing it
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
        // Test regeneration when messages vector is empty (edge case)
        let initial_messages = vec![];
        let mut state = TuiState::new(initial_messages, "test instructions".to_string());

        // TuiState::new should create a default message when initial_messages is empty
        assert_eq!(state.messages().len(), 1);
        assert_eq!(state.current_index(), 0);

        // Simulate regeneration result
        let new_message = GeneratedMessage {
            title: "New commit".to_string(),
            message: "New message".to_string(),
        };

        // This simulates the logic in the main loop
        state.add_message(new_message);

        // Verify the message was added
        assert_eq!(state.messages().len(), 2, "Should add new message");
        assert_eq!(state.current_index(), 1, "Should switch to new message");
        assert_eq!(state.messages()[1].title, "New commit");
    }

    #[test]
    fn test_regeneration_always_adds_message() {
        // Test that regeneration always adds a new message regardless of current_index
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

        // This simulates the logic in the main loop - always add new message
        state.add_message(new_message);

        // Should add the message
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
