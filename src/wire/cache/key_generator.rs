use std::hash::{DefaultHasher, Hash, Hasher};

use crate::wire::models::repo_config::RepositoryConfiguration;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::models::repo_config::RepositoryConfiguration;

    #[test]
    fn test_generate_key() {
        let config = RepositoryConfiguration::new(
            "https://github.com/example/repo.git".to_string(),
            "main".to_string(),
            "./src/module1".to_string(),
            vec!["src/".to_string()],
            None,
        );

        let key1 = CacheKeyGenerator::generate_key(&config);
        let key2 = CacheKeyGenerator::generate_key(&config);

        // Same config should produce same key
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_generate_different_keys_for_different_repos() {
        let config1 = RepositoryConfiguration::new(
            "https://github.com/example/repo1.git".to_string(),
            "main".to_string(),
            "./src/module1".to_string(),
            vec!["src/".to_string()],
            None,
        );

        let config2 = RepositoryConfiguration::new(
            "https://github.com/example/repo2.git".to_string(), // Different repo
            "main".to_string(),
            "./src/module1".to_string(),
            vec!["src/".to_string()],
            None,
        );

        let key1 = CacheKeyGenerator::generate_key(&config1);
        let key2 = CacheKeyGenerator::generate_key(&config2);

        // Different repos should produce different keys
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_generate_different_keys_for_different_branches() {
        let config1 = RepositoryConfiguration::new(
            "https://github.com/example/repo.git".to_string(),
            "main".to_string(), // Main branch
            "./src/module1".to_string(),
            vec!["src/".to_string()],
            None,
        );

        let config2 = RepositoryConfiguration::new(
            "https://github.com/example/repo.git".to_string(), // Same repo
            "develop".to_string(),                             // Different branch
            "./src/module1".to_string(),
            vec!["src/".to_string()],
            None,
        );

        let key1 = CacheKeyGenerator::generate_key(&config1);
        let key2 = CacheKeyGenerator::generate_key(&config2);

        // Different branches should produce different keys
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_generate_url_branch_key() {
        let key1 = CacheKeyGenerator::generate_url_branch_key(
            "https://github.com/example/repo.git",
            "main",
        );
        let key2 = CacheKeyGenerator::generate_url_branch_key(
            "https://github.com/example/repo.git",
            "main",
        );

        // Same URL and branch should produce same key
        assert_eq!(key1, key2);

        let key3 = CacheKeyGenerator::generate_url_branch_key(
            "https://github.com/example/repo.git",
            "develop", // Different branch
        );

        // Different branch should produce different key
        assert_ne!(key1, key3);
    }
}
