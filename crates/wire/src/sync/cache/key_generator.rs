use std::hash::{DefaultHasher, Hash, Hasher};

use crate::sync::models::repo_config::RepositoryConfiguration;

pub struct CacheKeyGenerator;

impl CacheKeyGenerator {
    /// Generate a unique cache key for a repository configuration
    /// The key is based on the repository URL and branch
    pub fn generate_key(config: &RepositoryConfiguration) -> String {
        let mut hasher = DefaultHasher::new();

        // Hash the URL and branch to create a unique key
        config.url.hash(&mut hasher);
        config.branch.hash(&mut hasher);

        // If commit hash is specified, include it in the key
        if let Some(ref commit) = config.commit_hash {
            commit.hash(&mut hasher);
        }

        let hash = hasher.finish();
        format!("{hash:x}")
    }

    /// Generate a cache key specifically for the URL and branch
    pub fn generate_url_branch_key(url: &str, branch: &str) -> String {
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        branch.hash(&mut hasher);

        let hash = hasher.finish();
        format!("{hash:x}")
    }
}
