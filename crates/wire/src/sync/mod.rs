pub mod cache;
pub mod common;
pub mod models;
pub mod wire;

pub use cache::manager::CacheManager;
pub use models::cached_repo::CachedRepository;
pub use models::repo_config::RepositoryConfiguration;
pub use models::wire_operation::WireOperation;
pub use wire::{check, operation::sync_with_caching};

pub fn init_logger() {
    env_logger::init();
}
