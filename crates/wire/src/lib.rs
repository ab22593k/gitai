pub mod sync;
pub mod sync_wire;

pub use sync::common::{Parsed, TargetConfig, infer_from_url, normalize_github_url, sequence};
pub use sync::{
    CacheManager, CachedRepository, RepositoryConfiguration, WireOperation, init_logger,
};
pub use sync_wire::handle_wire;
