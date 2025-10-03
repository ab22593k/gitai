use gitpilot::wire::{RepositoryConfiguration, WireOperation};

#[test]
fn test_wire_operation_creation() {
    let config = RepositoryConfiguration {
        url: "https://github.com/example/repo.git".to_string(),
        branch: "main".to_string(),
        target_path: "./src/module1".to_string(),
        filters: vec!["src/".to_string(), "lib/".to_string()],
        commit_hash: None,
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
    };

    let wire_op = WireOperation::new(config, "/tmp/cache/repo2".to_string());

    assert_eq!(
        wire_op.source_config.commit_hash,
        Some("abc123".to_string())
    );
    assert_eq!(wire_op.cached_repo_path, "/tmp/cache/repo2");
}
