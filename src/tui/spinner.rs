use crate::core::messages::{ColoredMessage, get_waiting_message};
use ratatui::style::Color;
use unicode_width::UnicodeWidthStr;

pub struct SpinnerState {
    frames: Vec<&'static str>,
    current_frame: usize,
    message: ColoredMessage,
}

impl Default for SpinnerState {
    fn default() -> Self {
        Self::new()
    }
}

impl SpinnerState {
    // Modern gradient-style spinner with smooth rotation
    pub fn new() -> Self {
        Self {
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            current_frame: 0,
            message: get_waiting_message().clone(),
        }
    }

    // Create spinner with custom message
    pub fn with_message(message: &str) -> Self {
        Self {
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            current_frame: 0,
            message: ColoredMessage {
                text: message.to_string(),
                color: ratatui::style::Color::Cyan, // Default color for custom messages
            },
        }
    }

    // Bouncing dots (like loading ellipsis)
    #[allow(dead_code)]
    pub fn with_dots() -> Self {
        Self {
            frames: vec!["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈"],
            current_frame: 0,
            message: get_waiting_message().clone(),
        }
    }

    // Growing circle (pulsing effect)
    #[allow(dead_code)]
    pub fn with_circle() -> Self {
        Self {
            frames: vec!["◜", "◠", "◝", "◞", "◡", "◟"],
            current_frame: 0,
            message: get_waiting_message().clone(),
        }
    }

    // Arrow rotation (directional feel)
    #[allow(dead_code)]
    pub fn with_arrows() -> Self {
        Self {
            frames: vec!["←", "↖", "↑", "↗", "→", "↘", "↓", "↙"],
            current_frame: 0,
            message: get_waiting_message().clone(),
        }
    }

    // Box corners (modern, geometric)
    #[allow(dead_code)]
    pub fn with_corners() -> Self {
        Self {
            frames: vec!["┤", "┘", "┴", "└", "├", "┌", "┬", "┐"],
            current_frame: 0,
            message: get_waiting_message().clone(),
        }
    }

    // Classic bar (minimalist)
    #[allow(dead_code)]
    pub fn with_bar() -> Self {
        Self {
            frames: vec!["-", "\\", "|", "/"],
            current_frame: 0,
            message: get_waiting_message().clone(),
        }
    }

    pub fn tick(&mut self) -> (String, String, Color, usize) {
        let frame = self.frames[self.current_frame];
        self.current_frame = (self.current_frame + 1) % self.frames.len();

        let spinner_with_space = format!("{frame} ");
        let width = spinner_with_space.width() + self.message.text.width();

        (
            spinner_with_space,
            self.message.text.clone(),
            self.message.color,
            width,
        )
    }
}
