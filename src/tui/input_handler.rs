use super::app::TuiCommit;
use super::spinner::SpinnerState;
use super::state::Mode;

pub trait TuiApp {
    fn get_state(&mut self) -> &mut super::state::TuiState;
}

impl TuiApp for TuiCommit {
    fn get_state(&mut self) -> &mut super::state::TuiState {
        &mut self.state
    }
}

use crate::features::commit::types::format_commit_message;
use crossterm::event::{KeyCode, KeyEvent};

pub async fn handle_input<A: TuiApp>(app: &mut A, key: KeyEvent) -> InputResult {
    let mode = app.get_state().mode.clone();
    match mode {
        Mode::Normal => {
            let result = handle_normal_mode(app, key);
            app.get_state().dirty = true; // Mark dirty after handling input
            result
        }
        Mode::EditingMessage => {
            let result = handle_editing_message(app, key);
            app.get_state().dirty = true; // Mark dirty after handling input
            result
        }
        Mode::EditingInstructions => handle_editing_instructions(app, key),
        Mode::Help => handle_help(app, key),
        Mode::Generating => {
            if key.code == KeyCode::Esc {
                let state = app.get_state();
                state.mode = Mode::Normal;
                state.set_status(String::from("Message generation cancelled."));
            }
            InputResult::Continue
        }
    }
}

fn handle_normal_mode<A: TuiApp>(app: &mut A, key: KeyEvent) -> InputResult {
    let state = app.get_state();
    match key.code {
        KeyCode::Char('e') => {
            state.mode = Mode::EditingMessage;
            state.set_status(String::from("Editing commit message. Press Esc to finish."));
            InputResult::Continue
        }
        KeyCode::Char('i') => {
            state.instructions_visible = !state.instructions_visible;
            if state.instructions_visible {
                state.mode = Mode::EditingInstructions;
                state.set_status(String::from("Editing instructions. Press Esc to finish."));
            } else {
                state.mode = Mode::Normal;
                state.set_status(String::from("Instructions hidden."));
            }
            InputResult::Continue
        }
        KeyCode::Char('R') => {
            // Regenerate is only for commit mode
            InputResult::Continue
        }
        KeyCode::Left | KeyCode::Char('l') => {
            if state.current_index > 0 {
                state.current_index -= 1;
            } else {
                state.current_index = state.messages.len() - 1;
            }
            state.update_message_textarea();
            state.set_status(format!(
                "Viewing commit message {}/{}",
                state.current_index + 1,
                state.messages.len()
            ));
            InputResult::Continue
        }
        KeyCode::Right | KeyCode::Char('r') => {
            if state.current_index < state.messages.len() - 1 {
                state.current_index += 1;
            } else {
                state.current_index = 0;
            }
            state.update_message_textarea();
            state.set_status(format!(
                "Viewing commit message {}/{}",
                state.current_index + 1,
                state.messages.len()
            ));
            InputResult::Continue
        }
        KeyCode::Enter => {
            let commit_message = format_commit_message(&state.messages[state.current_index]);
            state.set_status(String::from("Committing..."));
            state.spinner = Some(SpinnerState::new());

            InputResult::Commit(commit_message)
        }
        KeyCode::Char('?') => {
            state.nav_bar_visible = !state.nav_bar_visible;
            state.set_status(if state.nav_bar_visible {
                String::from("Navigation bar shown.")
            } else {
                String::from("Navigation bar hidden.")
            });
            InputResult::Continue
        }
        KeyCode::Char('h') => {
            state.mode = Mode::Help;
            state.set_status(String::from("Viewing help. Press any key to close."));
            InputResult::Continue
        }
        KeyCode::Esc => InputResult::Exit,
        _ => InputResult::Continue,
    }
}

fn handle_editing_message<A: TuiApp>(app: &mut A, key: KeyEvent) -> InputResult {
    let state = app.get_state();
    if key.code == KeyCode::Esc {
        state.mode = Mode::Normal;
        let edited_content = state.message_textarea.lines().join("\n");
        if let Some(message) = state.messages.get_mut(state.current_index) {
            // Split the edited content into title and message
            let mut lines = edited_content.lines();
            let title_line = lines.next().unwrap_or("").trim();
            message.title = title_line.to_string();

            // Collect the rest of the lines, skipping any leading empty lines
            message.message = lines
                .skip_while(|line| line.trim().is_empty())
                .collect::<Vec<&str>>()
                .join("\n");
        }
        state.set_status(String::from("Commit message updated."));
        state.update_message_textarea();
        InputResult::Continue
    } else {
        state.message_textarea.input(key);
        InputResult::Continue
    }
}

fn handle_editing_instructions<A: TuiApp>(app: &mut A, key: KeyEvent) -> InputResult {
    let state = app.get_state();
    if key.code == KeyCode::Esc {
        state.mode = Mode::Normal;
        state.custom_instructions = state.instructions_textarea.lines().join("\n");
        state.set_status(String::from("Instructions updated."));
        // Regenerate is only for commit mode
        InputResult::Continue
    } else {
        state.instructions_textarea.input(key);
        InputResult::Continue
    }
}

fn handle_help<A: TuiApp>(app: &mut A, _key: KeyEvent) -> InputResult {
    let state = app.get_state();
    state.mode = Mode::Normal; // Return to normal mode
    state.set_status(String::from("Help closed. Press '?' for help."));
    InputResult::Continue
}

pub enum InputResult {
    Continue,
    Exit,
    Commit(String),
}
