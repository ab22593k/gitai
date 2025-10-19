//! TUI module
//!
//! This module contains the TUI (Text User Interface) implementation.
//! It provides an interactive interface for users to generate and manage commit messages.

mod app;
mod input_handler;
pub mod spinner;
mod state;
mod ui;

pub use app::TuiCommit;
pub use app::run_tui_commit;
