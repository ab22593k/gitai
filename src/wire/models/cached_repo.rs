use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedRepository {
    /// The URL of the source repository
    pub url: String,
    /// The branch that was pulled
    pub branch: String,
    /// The path where the cached repository is stored
    pub local_cache_path: String,
    /// Timestamp of the last pull operation
    pub last_pulled: SystemTime,
    /// The commit hash of the cached repository
    pub commit_hash: String,
    // Using a simple boolean flag instead of a full mutex since we can't serialize mutex
    // The actual locking mechanism will be handled separately in the cache manager
    #[serde(skip)]
    pub in_use: Arc<Mutex<bool>>,
}

impl CachedRepository {
    pub fn new(url: String, branch: String, local_cache_path: String, commit_hash: String) -> Self {
        Self {
            url,
            branch,
            local_cache_path,
            last_pulled: SystemTime::now(),
            commit_hash,
            in_use: Arc::new(Mutex::new(false)),
        }
    }
}
