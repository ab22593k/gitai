use colored::Colorize;
use console::Term;
use parking_lot::Mutex;
use ratatui::style::Color;
use std::fmt::Write as _;

// Re-export SpinnerState for TUI use
pub use crate::tui::spinner::SpinnerState;

pub const STARLIGHT: Color = Color::Rgb(255, 255, 240);
pub const NEBULA_PURPLE: Color = Color::Rgb(167, 132, 239);
pub const CELESTIAL_BLUE: Color = Color::Rgb(75, 115, 235);
pub const SOLAR_YELLOW: Color = Color::Rgb(255, 225, 100);
pub const AURORA_GREEN: Color = Color::Rgb(140, 255, 170);
pub const PLASMA_CYAN: Color = Color::Rgb(20, 255, 255);
pub const METEOR_RED: Color = Color::Rgb(255, 89, 70);
pub const GALAXY_PINK: Color = Color::Rgb(255, 162, 213);
pub const COMET_ORANGE: Color = Color::Rgb(255, 165, 0);
pub const BLACK_HOLE: Color = Color::Rgb(0, 0, 0);

/// Track quiet mode state
static QUIET_MODE: std::sync::LazyLock<Mutex<bool>> =
    std::sync::LazyLock::new(|| Mutex::new(false));

/// Enable or disable quiet mode
#[inline]
pub fn set_quiet_mode(enabled: bool) {
    *QUIET_MODE.lock() = enabled;
}

/// Check if quiet mode is enabled
#[inline]
pub fn is_quiet_mode() -> bool {
    *QUIET_MODE.lock()
}

/// Create a TUI spinner state for terminal user interface
#[inline]
#[must_use]
pub fn create_tui_spinner(message: &str) -> SpinnerState {
    SpinnerState::with_message(message)
}

pub fn print_info(message: &str) {
    if !is_quiet_mode() {
        println!("{}", message.cyan().bold());
    }
}

pub fn print_warning(message: &str) {
    if !is_quiet_mode() {
        println!("{}", message.yellow().bold());
    }
}

pub fn print_error(message: &str) {
    // Always print errors, even in quiet mode
    eprintln!("{}", message.red().bold());
}

pub fn print_success(message: &str) {
    if !is_quiet_mode() {
        println!("{}", message.green().bold());
    }
}

/// Print content with decorative borders
pub fn print_bordered_content(content: &str) {
    if !is_quiet_mode() {
        let border = "━".repeat(50).bright_purple();
        println!("{border}");
        println!("{content}");
        println!("{border}");
    }
}

/// Print a simple message (respects quiet mode)
pub fn print_message(message: &str) {
    if !is_quiet_mode() {
        println!("{message}");
    }
}

/// Print an empty line (respects quiet mode)
pub fn print_newline() {
    if !is_quiet_mode() {
        println!();
    }
}

#[must_use]
pub fn create_gradient_text(text: &str) -> String {
    const GRADIENT: &[(u8, u8, u8)] = &[
        (129, 0, 255), // Deep purple
        (134, 51, 255),
        (139, 102, 255),
        (144, 153, 255),
        (149, 204, 255), // Light cyan
    ];

    apply_gradient(text, GRADIENT)
}

#[must_use]
pub fn create_secondary_gradient_text(text: &str) -> String {
    const GRADIENT: &[(u8, u8, u8)] = &[
        (75, 0, 130),   // Indigo
        (106, 90, 205), // Slate blue
        (138, 43, 226), // Blue violet
        (148, 0, 211),  // Dark violet
        (153, 50, 204), // Dark orchid
    ];

    apply_gradient(text, GRADIENT)
}

#[must_use]
fn apply_gradient(text: &str, gradient: &[(u8, u8, u8)]) -> String {
    let chars: Vec<char> = text.chars().collect();

    if chars.is_empty() || gradient.is_empty() {
        return String::new();
    }

    let chars_len = chars.len();
    let gradient_len = gradient.len();
    let mut result = String::with_capacity(text.len() * 20); // Reserve space for ANSI codes

    for (i, &c) in chars.iter().enumerate() {
        let index = if chars_len == 1 {
            0
        } else {
            i * (gradient_len - 1) / (chars_len - 1)
        };
        let (r, g, b) = gradient[index];
        let _ = write!(result, "{}", c.to_string().truecolor(r, g, b));
    }

    result
}

pub fn write_gradient_text(
    term: &Term,
    text: &str,
    gradient: &[(u8, u8, u8)],
) -> std::io::Result<()> {
    let gradient_text = apply_gradient(text, gradient);
    term.write_line(&gradient_text)
}

pub fn write_colored_text(term: &Term, text: &str, color: (u8, u8, u8)) -> std::io::Result<()> {
    let (r, g, b) = color;
    let colored_text = text.truecolor(r, g, b);
    term.write_line(&colored_text)
}

pub fn write_bold_text(term: &Term, text: &str) -> std::io::Result<()> {
    let bold_text = text.bold();
    term.write_line(&bold_text)
}
