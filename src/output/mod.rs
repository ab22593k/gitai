//! CLI output utilities.
//!
//! - Color constants and quiet mode (`colors`)
//! - Text formatting, gradients, terminal output (`formatting`)

mod colors;
mod formatting;

// Re-export SpinnerState for backward compatibility
pub use crate::tui::spinner::SpinnerState;

pub use colors::{
    AURORA_GREEN, BLACK_HOLE, CELESTIAL_BLUE, COMET_ORANGE, GALAXY_PINK, METEOR_RED, NEBULA_PURPLE,
    PLASMA_CYAN, SOLAR_YELLOW, STARLIGHT, is_quiet_mode, set_quiet_mode,
};

pub use formatting::{
    create_gradient_text, create_secondary_gradient_text, print_bordered_content, print_error,
    print_info, print_message, print_newline, print_success, print_warning, write_bold_text,
    write_colored_text, write_gradient_text,
};

/// Create a TUI spinner state for terminal user interface
#[inline]
#[must_use]
pub fn create_tui_spinner(message: &str) -> SpinnerState {
    SpinnerState::with_message(message)
}
