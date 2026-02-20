use gitai::{CachedRepository, RepositoryConfiguration, WireOperation};
use std::collections::HashMap;

#[test]
fn test_repository_deduplication_logic() {
    // Simulate a configuration with multiple entries for the same repository
    let configs = vec![
        RepositoryConfiguration {
            url: "https://github.com/example/repo.git".to_string(),
            branch: "main".to_string(),
            target_path: "./src/module1".to_string(),
            filters: vec!["src/".to_string(), "lib/".to_string()],
            commit_hash: None,
            mtd: None,
        },
        RepositoryConfiguration {
            url: "https://github.com/example/repo.git".to_string(), // Same repo
            branch: "main".to_string(),
            target_path: "./src/module2".to_string(),
            filters: vec!["utils/".to_string()],
            commit_hash: None,
            mtd: None,
        },
        RepositoryConfiguration {
            url: "https://github.com/other/repo.git".to_string(), // Different repo
            branch: "main".to_string(),
            target_path: "./src/module3".to_string(),
            filters: vec!["docs/".to_string()],
            commit_hash: None,
            mtd: None,
        },
    ];

    // This test verifies that we can identify duplicate repositories
    let mut repo_map: HashMap<String, Vec<RepositoryConfiguration>> = HashMap::new();

    for config in &configs {
        repo_map
            .entry(config.url.clone())
            .or_default()
            .push(config.clone());
    }

    // There should be 2 unique repositories (not 3)
    assert_eq!(repo_map.len(), 2);

    // The first repository should have 2 configurations
    assert_eq!(
        repo_map
            .get("https://github.com/example/repo.git")
            .expect("should have example repo entry")
            .len(),
        2
    );

    // The second repository should have 1 configuration
    assert_eq!(
        repo_map
            .get("https://github.com/other/repo.git")
            .expect("should have example repo entry")
            .len(),
        1
    );
}

#[test]
fn test_cached_repository_usage() {
    // Create a cached repository
    let cached_repo = CachedRepository::new(
        "https://github.com/example/repo.git".to_string(),
        "main".to_string(),
        "/tmp/cache/repo".to_string(),
        "abc123".to_string(),
    );

    // Create wire operations that would use this cached repository
    let config1 = RepositoryConfiguration {
        url: "https://github.com/example/repo.git".to_string(),
        branch: "main".to_string(),
        target_path: "./src/module1".to_string(),
        filters: vec!["src/".to_string()],
        commit_hash: None,
        mtd: None,
    };

    let config2 = RepositoryConfiguration {
        url: "https://github.com/example/repo.git".to_string(),
        branch: "main".to_string(),
        target_path: "./src/module2".to_string(),
        filters: vec!["utils/".to_string()],
        commit_hash: None,
        mtd: None,
    };

    let op1 = WireOperation::new(config1, cached_repo.local_cache_path.clone());
    let op2 = WireOperation::new(config2, cached_repo.local_cache_path.clone());

    // Both operations should reference the same cached repository path
    assert_eq!(op1.cached_repo_path, op2.cached_repo_path);
    assert_eq!(op1.cached_repo_path, "/tmp/cache/repo");
}
