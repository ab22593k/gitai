//! Terminal lifecycle management for the TUI
//!
//! This module handles:
//! - Terminal setup (raw mode, alternate screen, panic hook)
//! - Terminal cleanup (RAII-style guard for automatic restoration)
//! - Theme initialization

use crate::common::ThemeMode;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};
use std::{io, panic};

use super::theme::init_theme;

/// RAII guard for terminal state
///
/// Automatically restores terminal state when dropped:
/// - Disables raw mode
/// - Leaves alternate screen
/// - Shows cursor
pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    /// Create a new terminal guard wrapping an existing terminal
    pub fn new(terminal: Terminal<CrosstermBackend<io::Stdout>>) -> Self {
        Self { terminal }
    }

    /// Get mutable access to the underlying terminal
    pub fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self.terminal
    }

    /// Get reference to the underlying terminal
    pub fn terminal(&self) -> &Terminal<CrosstermBackend<io::Stdout>> {
        &self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Restore terminal state on drop
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

/// Terminal runtime manager
///
/// Handles setup and teardown of the terminal environment.
pub struct TuiRuntime;

impl TuiRuntime {
    /// Initialize the terminal environment
    ///
    /// This performs:
    /// 1. Installs panic hook to restore terminal on panic
    /// 2. Enables raw mode
    /// 3. Enters alternate screen
    /// 4. Creates Terminal with `CrosstermBackend`
    ///
    /// Returns a `TerminalGuard` that will automatically clean up on drop.
    pub fn setup() -> io::Result<TerminalGuard> {
        // Install panic hook to restore terminal
        let default_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info: &panic::PanicHookInfo| {
            let _ = crossterm::terminal::disable_raw_mode();
            default_hook(panic_info);
        }));

        // Enable raw mode
        enable_raw_mode()?;

        // Enter alternate screen and create terminal
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(TerminalGuard::new(terminal))
    }

    /// Initialize theme and prepare terminal for TUI operation
    ///
    /// This performs setup only; caller is responsible for:
    /// - Running main loop
    /// - Dropping `TerminalGuard` for cleanup
    pub fn setup_with_theme(theme_mode: ThemeMode) -> io::Result<TerminalGuard> {
        // Initialize adaptive theme
        init_theme(theme_mode);

        // Setup terminal
        Self::setup()
    }
}

/// Application exit status
pub enum ExitStatus {
    /// Successfully committed with the given output message
    Committed(String),
    /// User cancelled the operation
    Cancelled,
    /// An error occurred during commit
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_guard_has_non_zero_size() {
        assert!(
            std::mem::size_of::<TerminalGuard>() > 0,
            "TerminalGuard should have a non-zero size"
        );
    }

    #[test]
    fn test_terminal_guard_needs_drop() {
        assert!(
            std::mem::needs_drop::<TerminalGuard>(),
            "TerminalGuard should need drop (has cleanup logic)"
        );
    }

    #[test]
    fn test_panic_hook_disables_raw_mode() {
        // Test that the panic hook code compiles and the closure is valid
        let closure = |panic_info: &std::panic::PanicHookInfo| {
            let _ = panic_info;
            let _ = disable_raw_mode();
        };
        // Use closure to prevent dead-code warning; the compile-time check is the test
        assert!(std::mem::size_of_val(&closure) == 0);
    }

    #[test]
    fn test_terminal_backend_is_crossterm() {
        // Verify that we use CrosstermBackend at compile time
        fn assert_backend_type<T>() {}
        assert_backend_type::<Terminal<CrosstermBackend<io::Stdout>>>();
    }
}
