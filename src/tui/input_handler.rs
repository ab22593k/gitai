use super::state::{Mode, TuiState};
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, PartialEq, Eq)]
pub enum InputResult {
    Continue,
    Exit,
    Commit(String),
}

pub async fn handle_input(tui: &mut super::app::TuiCommit, key: KeyEvent) -> InputResult {
    let state = &mut tui.state;

    match state.mode() {
        Mode::Normal => handle_normal_mode(state, key),
        Mode::EditingMessage => handle_editing_message_mode(state, key),
        Mode::EditingInstructions => handle_editing_instructions_mode(state, key),
        Mode::Generating => InputResult::Continue, // Ignore input while generating
        Mode::Help => handle_help_mode(state, key),
        Mode::Completing => handle_completing_mode(state, key),
        Mode::ContextSelection => handle_context_selection_mode(state, key),
    }
}

#[allow(clippy::match_same_arms)]
fn handle_normal_mode(state: &mut TuiState, key: KeyEvent) -> InputResult {
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
        KeyCode::Char('e') => {
            state.set_mode(Mode::EditingMessage);
            state.set_status(
                "Editing commit message... Press 'Esc' to finish, 'Tab' for completion",
            );
            InputResult::Continue
        }
        KeyCode::Char('i') => {
            state.set_mode(Mode::EditingInstructions);
            state.set_status("Editing instructions... Press 'Esc' to finish");
            InputResult::Continue
        }
        KeyCode::Char('r') => {
            state.set_mode(Mode::Generating);
            state.set_spinner(Some(super::spinner::SpinnerState::new()));
            state.set_status("Regenerating commit message...");
            state.set_dirty(true);
            InputResult::Continue
        }
        KeyCode::Char('?') => {
            state.set_mode(Mode::Help);
            InputResult::Continue
        }
        KeyCode::Char('c') => {
            state.set_mode(Mode::ContextSelection);
            state.set_status(
                "Select context: 'Space' toggle, 'Tab' switch category, 'Enter' confirm, 'Esc' cancel",
            );
            InputResult::Continue
        }
        KeyCode::Left => {
            state.previous_message();
            state.set_status(format!(
                "Viewing message {}/{}",
                state.current_index() + 1,
                state.messages().len()
            ));
            InputResult::Continue
        }
        KeyCode::Right => {
            state.next_message();
            state.set_status(format!(
                "Viewing message {}/{}",
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
    match key.code {
        KeyCode::Esc => {
            state.set_mode(Mode::Normal);
            state.update_current_message_from_textarea();
            state.set_status("Edited message saved. Press 'Enter' to commit.");
            InputResult::Continue
        }
        KeyCode::Tab => {
            // Get the text before the cursor to use as prefix for completion
            let (row, col) = state.message_textarea().cursor();
            let lines = state.message_textarea().lines();
            if row < lines.len() {
                let line = &lines[row];
                if col <= line.len() {
                    let prefix = line[..col].to_string();
                    // Only trigger completion if we have some text and it's not just whitespace
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
    if key.code == KeyCode::Esc {
        state.set_mode(Mode::Normal);
        state.update_instructions_from_textarea();
        state.set_status("Instructions updated. Press 'R' to regenerate.");
        InputResult::Continue
    } else {
        state.instructions_textarea_mut().input(key);
        state.set_dirty(true);
        InputResult::Continue
    }
}

fn handle_help_mode(state: &mut TuiState, _key: KeyEvent) -> InputResult {
    state.set_mode(Mode::Normal); // Return to normal mode
    state.set_status("Press '?': help | 'Esc': exit");
    InputResult::Continue
}

fn handle_completing_mode(state: &mut TuiState, key: KeyEvent) -> InputResult {
    match key.code {
        KeyCode::Esc => {
            state.set_mode(Mode::EditingMessage);
            state.set_status("Completion cancelled.");
            state.set_completion_suggestions(Vec::new()); // Clear suggestions
            InputResult::Continue
        }
        KeyCode::Enter => {
            // Apply the selected suggestion
            if !state.completion_suggestions().is_empty() {
                let suggestion = state.completion_suggestions()[state.completion_index()].clone();
                state.message_textarea_mut().insert_str(&suggestion);
                state.set_completion_suggestions(Vec::new()); // Clear suggestions
                state.set_mode(Mode::EditingMessage);
                state.set_status("Completion applied.");
            }
            InputResult::Continue
        }
        KeyCode::Tab | KeyCode::Down => {
            // Cycle forward through suggestions
            state.next_completion();
            InputResult::Continue
        }
        KeyCode::BackTab | KeyCode::Up => {
            // Cycle backward through suggestions
            state.previous_completion();
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}

fn handle_context_selection_mode(state: &mut TuiState, key: KeyEvent) -> InputResult {
    match key.code {
        KeyCode::Esc => {
            state.set_mode(Mode::Normal);
            state.set_status("Context selection cancelled.");
            InputResult::Continue
        }
        KeyCode::Enter => {
            state.set_mode(Mode::Normal);
            state.set_status("Context updated. Press 'R' to regenerate with new context.");
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
