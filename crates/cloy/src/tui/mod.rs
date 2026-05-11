//! TUI module
//!
//! This module contains the TUI (Text User Interface) implementation.
//! It provides an interactive interface for users to generate and manage commit messages.
//!
//! # Architecture
//!
//! The TUI is split into focused components:
//! - [`TuiCommit`] - Main coordinator, owns state and services
//! - [`TuiRuntime`] - Terminal lifecycle management (setup/teardown)
//! - [`TuiTaskRunner`] - Async task spawning for generation/completion
//! - `renderer` - Pure UI rendering
//! - [`TuiState`] - Pure UI model (state management)
//! - `input` - Key event dispatch and mode-specific handlers

mod coordinator;
mod input;
mod renderer;
mod runtime;
pub mod spinner;
mod state;
mod task_runner;
pub mod theme;

pub use coordinator::TuiCommit;
pub use coordinator::run_tui_commit;
pub use runtime::{ExitStatus, TuiRuntime};
pub use state::TuiState;
pub use theme::Theme;
