pub mod analyzer;
pub mod app;
pub mod common;
pub mod config;
pub mod core;
pub mod features;
pub mod git;
pub mod logger;
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
pub use features::commit::{
    review::{CodeIssue, DimensionAnalysis, GeneratedReview, QualityDimension},
    types::{GeneratedMessage, GeneratedPullRequest, format_commit_message, format_pull_request},
};

// Re-exports from wire
pub use remote::{
    CacheManager, CachedRepository, RepositoryConfiguration, WireOperation, init_logger,
};
