use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use super::key_generator::CacheKeyGenerator;
use crate::remote::models::{
    cached_repo::CachedRepository, repo_config::RepositoryConfiguration,
    wire_operation::WireOperation,
};

type CacheKey = String;

#[derive(Default)]
pub struct CacheManager {
    // Maps cache key (hash of URL + branch + optional commit) to cached repository info
    cache: Arc<Mutex<HashMap<CacheKey, CachedRepository>>>,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Determine if we need to fetch a repository, or if it's already cached.
    /// Returns the cache path for the repository.
    pub fn get_or_schedule_fetch(
        &self,
        config: &RepositoryConfiguration,
    ) -> Result<String, String> {
        let key = CacheKeyGenerator::generate_key(config);
        let Ok(cache) = self.cache.lock() else {
            return Err(
                "Cache mutex was poisoned. Cache may be in inconsistent state.".to_string(),
            );
        };

        if let Some(cached_repo) = cache.get(&key) {
            // Repository is already cached, return its path
            Ok(cached_repo.local_cache_path.clone())
        } else {
            // Repository is not cached yet, create a simulated cache entry
            // In a real implementation, this would involve fetching the repo
            let cache_path = Self::get_cache_path(&key)?;

            let new_cached_repo = CachedRepository::new(
                config.url.clone(),
                config.branch.clone(),
                cache_path.clone(),
                "placeholder_commit_hash".to_string(), // This would be obtained from the actual fetch
            );

            drop(cache); // Release lock before inserting
            let Ok(mut cache) = self.cache.lock() else {
                return Err("Cache mutex was poisoned after fetch operation.".to_string());
            };
            cache.insert(key, new_cached_repo);

            // Return the cache path for this repository
            Ok(cache_path)
        }
    }

    /// Get a unique cache path for a given cache key.
    fn get_cache_path(key: &str) -> Result<String, String> {
        let cache_dir = std::env::temp_dir().join("git-wire-cache").join(key);

        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to create cache directory: {e}"))?;

        Ok(cache_dir.to_string_lossy().to_string())
    }

    /// Process a list of repository configurations to determine the optimal fetching strategy.
    /// Identifies unique repositories based on URL, branch, and optional commit hash.
    /// Returns a list of unique configurations to fetch and corresponding wire operations.
    pub fn plan_fetch_operations(
        &self,
        configs: &[RepositoryConfiguration],
    ) -> Result<(Vec<RepositoryConfiguration>, Vec<WireOperation>), String> {
        // Identify unique repositories using cache keys
        let mut unique_configs: Vec<RepositoryConfiguration> = Vec::new();
        let mut seen_keys: HashSet<String> = HashSet::new();
        let mut operations: Vec<WireOperation> = Vec::new();

        for config in configs {
            let key = CacheKeyGenerator::generate_key(config);

            // Only add to unique configs if we haven't seen this key before
            if !seen_keys.contains(&key) {
                // This is a new unique repository (based on URL + branch + commit), add to fetch list
                unique_configs.push(config.clone());
                seen_keys.insert(key);
            }

            // Get or create cached path for this repository
            // Reuse the key we already generated instead of regenerating
            let cached_path = self.get_or_schedule_fetch(config)?;

            // Create a wire operation to extract content from the cached repo
            operations.push(WireOperation::new(config.clone(), cached_path));
        }

        Ok((unique_configs, operations))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_manager_creation() {
        let cache_manager = CacheManager::new();
        assert_eq!(
            cache_manager
                .cache
                .lock()
                .map(|guard| guard.len())
                .unwrap_or(0),
            0
        );
    }

    #[test]
    fn test_get_or_schedule_fetch_new_repo() {
        let cache_manager = CacheManager::new();
        let config = RepositoryConfiguration::new(
            "https://github.com/example/repo.git".to_string(),
            "main".to_string(),
            "./src/module1".to_string(),
            vec!["src/".to_string(), "lib/".to_string()],
            None,
            None,
        );

        let result = cache_manager.get_or_schedule_fetch(&config);
        assert!(result.is_ok());

        // Check that the repo was added to cache using the generated key
        let key = CacheKeyGenerator::generate_key(&config);
        let cache = cache_manager
            .cache
            .lock()
            .expect("Failed to lock cache in test");
        assert!(cache.contains_key(&key));
    }

    #[test]
    fn test_plan_fetch_operations_with_duplicates() {
        let cache_manager = CacheManager::new();

        // Create configs with duplicate repositories (same URL and branch)
        let configs = vec![
            RepositoryConfiguration::new(
                "https://github.com/example/repo.git".to_string(),
                "main".to_string(),
                "./src/module1".to_string(),
                vec!["src/".to_string(), "lib/".to_string()],
                None,
                None,
            ),
            RepositoryConfiguration::new(
                "https://github.com/example/repo.git".to_string(), // Same repo and branch
                "main".to_string(),
                "./src/module2".to_string(),
                vec!["utils/".to_string()],
                None,
                None,
            ),
            RepositoryConfiguration::new(
                "https://github.com/other/repo.git".to_string(), // Different repo
                "main".to_string(),
                "./src/module3".to_string(),
                vec!["docs/".to_string()],
                None,
                None,
            ),
        ];

        let (unique_configs, operations) = cache_manager
            .plan_fetch_operations(&configs)
            .expect("Failed to plan fetch operations");

        // Should only have 2 unique configs (not 3, since first two share URL + branch)
        assert_eq!(unique_configs.len(), 2);

        // Should have 3 operations (one for each original config)
        assert_eq!(operations.len(), 3);
    }
}
