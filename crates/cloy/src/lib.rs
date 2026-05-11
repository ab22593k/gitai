pub mod app;
pub mod commands;
pub mod common;
pub mod config;
pub mod git;
pub mod llm;
pub mod output;
pub mod tui;

pub use ::llm::LLMProvider;
pub use config::Config;
pub use config::ProviderConfig;
pub use llm::context::FixedSizeBuffer;

pub use commands::commit::types::{GeneratedMessage, format_commit_message};

pub use llm::engine::init_tracing_to_file;

pub fn init_app() {
    env_logger::init();
    init_tracing_to_file();
}
