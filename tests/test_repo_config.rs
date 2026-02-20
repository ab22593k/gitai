use gitai::RepositoryConfiguration;

#[test]
fn test_repository_configuration_creation() {
    let config = RepositoryConfiguration {
        url: "https://github.com/example/repo.git".to_string(),
        branch: "main".to_string(),
        target_path: "./src/module1".to_string(),
        filters: vec!["src/".to_string(), "lib/".to_string()],
        commit_hash: None,
        mtd: None,
    };

    assert_eq!(config.url, "https://github.com/example/repo.git");
    assert_eq!(config.branch, "main");
    assert_eq!(config.target_path, "./src/module1");
    assert_eq!(config.filters, vec!["src/".to_string(), "lib/".to_string()]);
    assert_eq!(config.commit_hash, None);
}

#[test]
fn test_repository_configuration_with_commit_hash() {
    let config = RepositoryConfiguration {
        url: "https://github.com/example/repo.git".to_string(),
        branch: "main".to_string(),
        target_path: "./src/module2".to_string(),
        filters: vec!["utils/".to_string()],
        commit_hash: Some("abc123def456".to_string()),
        mtd: None,
    };

    assert_eq!(config.commit_hash, Some("abc123def456".to_string()));
}

#[test]
fn test_repository_configuration_default_branch() {
    let config = RepositoryConfiguration {
        url: "https://github.com/example/repo.git".to_string(),
        branch: "main".to_string(), // default branch
        target_path: "./src/module1".to_string(),
        filters: vec![],
        commit_hash: None,
        mtd: None,
    };

    assert_eq!(config.branch, "main");
}
