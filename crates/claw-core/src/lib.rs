pub mod app;
pub mod commands;
pub mod common;
pub mod config;
pub mod git;
pub mod llm;
pub mod output;
pub mod sync;
pub mod tui;

pub use crate::app::{App, Gitai, handle_command, parse_args};

pub use ::llm::LLMProvider;
pub use config::Config;
pub use config::ProviderConfig;
pub use llm::context::FixedSizeBuffer;

pub use commands::commit::types::{
    GeneratedMessage, GeneratedPullRequest, format_commit_message, format_pull_request,
};

pub use sync::common::Parsed;
pub use sync::common::parse::{parse_gitwire, save_to_gitwire};
pub use sync::{
    CacheManager, CachedRepository, RepositoryConfiguration, WireOperation, init_logger,
};

pub use llm::engine::init_tracing_to_file;

pub fn init_app() {
    init_logger();
    init_tracing_to_file();
}
