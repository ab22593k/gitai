//! Text formatting utilities for CLI output.
//!
//! Gradient rendering, terminal writing, and bordered content.

use colored::Colorize;
use console::Term;
use std::fmt::Write as _;

use super::colors::is_quiet_mode;

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
        // Expressive: Thicker border with custom brand color
        let border = "━".repeat(60).truecolor(167, 132, 239); // NEBULA_PURPLE
        println!("{border}");
        println!(" {content} ");
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
        (255, 0, 127),   // Vibrant Pink
        (167, 132, 239), // Nebula Purple
        (75, 115, 235),  // Celestial Blue
        (20, 255, 255),  // Plasma Cyan
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
    let mut result = String::with_capacity(text.len() * 20);
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
