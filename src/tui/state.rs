use super::spinner::SpinnerState;
use crate::features::commit::types::{GeneratedMessage, format_commit_message};
use crate::instruction_presets::{get_instruction_preset_library, list_presets_formatted};

use ratatui::widgets::ListState;
use tui_textarea::TextArea;

#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    EditingMessage,
    EditingInstructions,
    SelectingPreset,
    Generating,
    Help,
}

pub struct TuiState {
    pub messages: Vec<GeneratedMessage>,
    pub current_index: usize,
    pub custom_instructions: String,
    pub status: String,
    pub selected_preset: String,
    pub mode: Mode,
    pub message_textarea: TextArea<'static>,
    pub instructions_textarea: TextArea<'static>,
    pub preset_list: Vec<(String, String, String, String)>,
    pub preset_list_state: ListState,
    pub spinner: Option<SpinnerState>,
    pub dirty: bool,
    pub last_spinner_update: std::time::Instant,
    pub instructions_visible: bool,
    pub nav_bar_visible: bool,
}

impl TuiState {
    pub fn new(
        initial_messages: Vec<GeneratedMessage>,
        custom_instructions: String,
        preset: String,
        user_name: String,
        user_email: String,
    ) -> Self {
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

        let preset_library = get_instruction_preset_library();
        let preset_list = list_presets_formatted(&preset_library)
            .split('\n')
            .map(|line| {
                let parts: Vec<String> = line
                    .split(" - ")
                    .map(std::string::ToString::to_string)
                    .collect();
                (
                    parts[0].clone(),
                    parts[1].clone(),
                    parts[2].clone(),
                    parts.get(3).cloned().unwrap_or_default(),
                )
            })
            .collect();

        let mut preset_list_state = ListState::default();
        preset_list_state.select(Some(0));

        let mut user_name_textarea = TextArea::default();
        user_name_textarea.insert_str(&user_name);
        let mut user_email_textarea = TextArea::default();
        user_email_textarea.insert_str(&user_email);

        Self {
            messages,
            current_index: 0,
            custom_instructions,
            status: "Press '?': help | 'Esc': exit".to_string(),
            selected_preset: preset,
            mode: Mode::Normal,
            message_textarea,
            instructions_textarea,
            preset_list,
            preset_list_state,
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
