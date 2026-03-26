pub mod app;
pub mod commands;
#[allow(
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]
pub mod common;
pub mod config;
pub mod git;
pub mod llm;
pub mod sync;
pub mod tui;
pub mod ui;

// Re-export important structs and functions for easier testing
pub use config::Config;
pub use config::ProviderConfig;
// Re-export the LLMProvider trait from the external llm crate
pub use ::llm::LLMProvider;
// Re-export the FixedSizeBuffer from core
pub use llm::context::FixedSizeBuffer;

// Re-exports from the new types organization
pub use commands::commit::types::{
    GeneratedMessage, GeneratedPullRequest, format_commit_message, format_pull_request,
};

// Re-exports from wire
pub use sync::common::Parsed;
pub use sync::common::parse::{parse_gitwire, save_to_gitwire};
pub use sync::{
    CacheManager, CachedRepository, RepositoryConfiguration, WireOperation, init_logger,
};

// Re-export tracing initialization
pub use llm::engine::init_tracing_to_file;

pub fn init_app() {
    init_logger();
    init_tracing_to_file();
}
