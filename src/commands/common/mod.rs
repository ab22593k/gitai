#![allow(clippy::uninlined_format_args)]

pub mod service;
pub mod spinner;
pub mod validation;

pub use service::{create_commit_service, create_completion_service};
pub use spinner::run_with_spinner;
pub use validation::{validate_context_ratio, validate_staged_files};
