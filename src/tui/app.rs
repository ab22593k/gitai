use super::input_handler::{InputResult, handle_input};
use super::spinner::SpinnerState;
use super::state::{Mode, TuiState};
use super::ui::draw_ui;
use crate::debug;
use crate::features::commit::{CommitService, format_commit_result, types::GeneratedMessage};
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

use std::io;
use std::sync::Arc;
use std::time::Duration;

pub struct TuiCommit {
    pub state: TuiState,
    service: Arc<CommitService>,
}

impl TuiCommit {
    pub fn new(
        initial_messages: Vec<GeneratedMessage>,
        custom_instructions: String,
        service: Arc<CommitService>,
    ) -> Self {
        let state = TuiState::new(initial_messages, custom_instructions);

        Self { state, service }
    }

    #[allow(clippy::unused_async)]
    pub async fn run(
        initial_messages: Vec<GeneratedMessage>,
        custom_instructions: String,
        service: Arc<CommitService>,
    ) -> Result<()> {
        let mut app = Self::new(initial_messages, custom_instructions, service);

        app.run_app().map_err(Error::from)
    }

    pub fn run_app(&mut self) -> io::Result<()> {
        // Setup
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run main loop
        let result = self.main_loop(&mut terminal);

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

    fn main_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> anyhow::Result<ExitStatus> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<GeneratedMessage, anyhow::Error>>(1);
        let mut task_spawned = false;

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
                let tx = tx.clone();

                tokio::spawn(async move {
                    debug!("Generating message...");
                    let result = service.generate_message(&instructions).await;
                    let _ = tx.send(result).await;
                });

                task_spawned = true; // Ensure we only spawn the task once
            }

            // Check if a message has been received from the generation task
            match rx.try_recv() {
                Ok(result) => match result {
                    Ok(new_message) => {
                        self.state.messages.push(new_message);
                        self.state.current_index = self.state.messages.len() - 1;

                        self.state.update_message_textarea();
                        self.state.mode = Mode::Normal; // Exit Generating mode
                        self.state.spinner = None; // Stop the spinner
                        self.state
                            .set_status(String::from("New message generated successfully!"));
                        task_spawned = false; // Reset for future regenerations
                    }
                    Err(e) => {
                        self.state.mode = Mode::Normal; // Exit Generating mode
                        self.state.spinner = None; // Stop the spinner
                        self.state
                            .set_status(format!("Failed to generate new message: {e}. Press 'r' to retry or 'Esc' to exit."));
                        task_spawned = false; // Reset for future regenerations
                    }
                },
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    // No message available yet, continue the loop
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    // Handle the case where the sender has disconnected
                    break;
                }
            }

            // Poll for input events
            if event::poll(Duration::from_millis(20))?
                && let Event::Key(key) = event::read()?
                && key.kind == crossterm::event::KeyEventKind::Press
            {
                match handle_input(self, key) {
                    InputResult::Exit => return Ok(ExitStatus::Cancelled),
                    InputResult::Commit(message) => match self.perform_commit(&message) {
                        Ok(status) => return Ok(status),
                        Err(e) => {
                            self.state.set_status(format!("Commit failed: {e}"));
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
    }

    pub fn perform_commit(&self, message: &str) -> Result<ExitStatus, Error> {
        match self.service.perform_commit(message) {
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
) -> Result<()> {
    TuiCommit::run(initial_messages, custom_instructions, service).await
}

pub enum ExitStatus {
    Committed(String),
    Cancelled,
    Error(String),
}
