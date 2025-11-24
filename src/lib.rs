pub mod app;
#[allow(
    clippy::uninlined_format_args,
    clippy::cast_possible_truncation,
    clippy::as_conversions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]
pub mod common;
pub mod config;
pub mod core;
pub mod features;
pub mod git;
pub mod remote;
pub mod tui;
pub mod ui;

// Re-export important structs and functions for easier testing
pub use config::Config;
pub use config::ProviderConfig;
// Re-export the LLMProvider trait from the external llm crate
pub use ::llm::LLMProvider;
// Re-export the FixedSizeBuffer from core
pub use core::context::FixedSizeBuffer;

// Re-exports from the new types organization
pub use features::commit::types::{
    GeneratedMessage, GeneratedPullRequest, format_commit_message, format_pull_request,
};

// Re-exports from wire
pub use remote::{
    CacheManager, CachedRepository, RepositoryConfiguration, WireOperation, init_logger,
};

// Re-export tracing initialization
pub use core::llm::init_tracing_to_file;
