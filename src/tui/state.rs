use super::spinner::SpinnerState;
use crate::features::commit::types::{GeneratedMessage, format_commit_message};


use tui_textarea::TextArea;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Mode {
    Normal,
    EditingMessage,
    EditingInstructions,
    Generating,
    Help,
}

pub struct TuiState {
    pub messages: Vec<GeneratedMessage>,
    pub current_index: usize,
    pub custom_instructions: String,
    pub status: String,
    pub mode: Mode,
    pub message_textarea: TextArea<'static>,
    pub instructions_textarea: TextArea<'static>,
    pub spinner: Option<SpinnerState>,
    pub dirty: bool,
    pub last_spinner_update: std::time::Instant,
    pub instructions_visible: bool,
    pub nav_bar_visible: bool,
}

impl TuiState {
    pub fn new(initial_messages: Vec<GeneratedMessage>, custom_instructions: String) -> Self {
        let mut message_textarea = TextArea::default();
        let messages = if initial_messages.is_empty() {
            vec![GeneratedMessage {
                emoji: None,
                title: String::new(),
                message: String::new(),
            }]
        } else {
            initial_messages
        };
        if let Some(first_message) = messages.first() {
            message_textarea.insert_str(format_commit_message(first_message));
        }

        let mut instructions_textarea = TextArea::default();
        instructions_textarea.insert_str(&custom_instructions);

        Self {
            messages,
            current_index: 0,
            custom_instructions,
            status: "Press '?': help | 'Esc': exit".to_string(),
            mode: Mode::Normal,
            message_textarea,
            instructions_textarea,
            spinner: None,
            dirty: true,
            last_spinner_update: std::time::Instant::now(),
            instructions_visible: false,
            nav_bar_visible: true,
        }
    }

    pub fn set_status(&mut self, new_status: String) {
        self.status = new_status;
        self.spinner = None;
        self.dirty = true;
    }

    pub fn update_message_textarea(&mut self) {
        let current_message = &self.messages[self.current_index];
        let message_content = format!(
            "{}\n\n{}",
            current_message.title,
            current_message.message.trim()
        );

        let mut new_textarea = TextArea::default();
        new_textarea.insert_str(&message_content);
        self.message_textarea = new_textarea;
        self.dirty = true;
    }


}
