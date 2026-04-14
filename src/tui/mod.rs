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
//! - [`TuiRenderer`] - Pure UI rendering (delegates to renderer module)
//! - [`TuiState`] - Pure UI model (state management)

mod app;
mod renderer;
mod runtime;
pub mod spinner;
mod state;
mod task_runner;
pub mod theme;

pub use app::TuiCommit;
pub use app::run_tui_commit;
pub use runtime::{ExitStatus, TuiRuntime};
pub use state::TuiState;
pub use theme::Theme;
