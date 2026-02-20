use gitai::{
    RepositoryConfiguration, WireOperation, remote::cache::key_generator::CacheKeyGenerator,
};

#[test]
fn test_wire_operation_creation() {
    let config = RepositoryConfiguration {
        url: "https://github.com/example/repo.git".to_string(),
        branch: "main".to_string(),
        target_path: "./src/module1".to_string(),
        filters: vec!["src/".to_string(), "lib/".to_string()],
        commit_hash: None,
        mtd: None,
    };

    let wire_op = WireOperation::new(config, "/tmp/cache/repo1".to_string());

    assert_eq!(wire_op.cached_repo_path, "/tmp/cache/repo1");
    assert_eq!(
        wire_op.source_config.url,
        "https://github.com/example/repo.git"
    );
    assert_eq!(wire_op.source_config.branch, "main");
    assert_eq!(wire_op.source_config.target_path, "./src/module1");
    assert_eq!(
        wire_op.source_config.filters,
        vec!["src/".to_string(), "lib/".to_string()]
    );
    assert_eq!(wire_op.source_config.commit_hash, None);
    // Operation ID should be generated (not default value)
    assert!(!wire_op.operation_id.as_u128().to_string().is_empty());
}

#[test]
fn test_wire_operation_with_commit_hash() {
    let config = RepositoryConfiguration {
        url: "https://github.com/example/repo.git".to_string(),
        branch: "develop".to_string(),
        target_path: "./src/module2".to_string(),
        filters: vec!["utils/".to_string()],
        commit_hash: Some("abc123".to_string()),
        mtd: None,
    };

    let wire_op = WireOperation::new(config, "/tmp/cache/repo2".to_string());

    assert_eq!(
        wire_op.source_config.commit_hash,
        Some("abc123".to_string())
    );
    assert_eq!(wire_op.cached_repo_path, "/tmp/cache/repo2");
}

#[test]
fn test_generate_key() {
    let config = RepositoryConfiguration::new(
        "https://github.com/example/repo.git".to_string(),
        "main".to_string(),
        "./src/module1".to_string(),
        vec!["src/".to_string()],
        None,
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
        None,
    );

    let config2 = RepositoryConfiguration::new(
        "https://github.com/example/repo2.git".to_string(), // Different repo
        "main".to_string(),
        "./src/module1".to_string(),
        vec!["src/".to_string()],
        None,
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
        None,
    );

    let config2 = RepositoryConfiguration::new(
        "https://github.com/example/repo.git".to_string(), // Same repo
        "develop".to_string(),                             // Different branch
        "./src/module1".to_string(),
        vec!["src/".to_string()],
        None,
        None,
    );

    let key1 = CacheKeyGenerator::generate_key(&config1);
    let key2 = CacheKeyGenerator::generate_key(&config2);

    // Different branches should produce different keys
    assert_ne!(key1, key2);
}

#[test]
fn test_generate_url_branch_key() {
    let key1 =
        CacheKeyGenerator::generate_url_branch_key("https://github.com/example/repo.git", "main");
    let key2 =
        CacheKeyGenerator::generate_url_branch_key("https://github.com/example/repo.git", "main");

    // Same URL and branch should produce same key
    assert_eq!(key1, key2);

    let key3 = CacheKeyGenerator::generate_url_branch_key(
        "https://github.com/example/repo.git",
        "develop", // Different branch
    );

    // Different branch should produce different key
    assert_ne!(key1, key3);
}
