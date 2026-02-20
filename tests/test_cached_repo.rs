use gitai::CachedRepository;

const EXAMPLE_REPO_URL: &str = "https://github.com/example/repo.git";

#[test]
fn test_cached_repository_creation() {
    let repo_url = EXAMPLE_REPO_URL.to_string();
    let branch = "main".to_string();
    let cache_path = "/tmp/cache/repo1".to_string();
    let commit_hash = "abc123".to_string();

    let cached_repo = CachedRepository::new(repo_url, branch, cache_path, commit_hash);

    assert_eq!(cached_repo.url, EXAMPLE_REPO_URL);
    assert_eq!(cached_repo.branch, "main");
    assert_eq!(cached_repo.local_cache_path, "/tmp/cache/repo1");
    assert_eq!(cached_repo.commit_hash, "abc123");
    // Test that last_pulled is set to the current time (approximately)
    assert!(
        cached_repo
            .last_pulled
            .elapsed()
            .expect("Failed to get elapsed time")
            .as_secs()
            < 1
    );
}

#[test]
fn test_cached_repository_with_different_branch() {
    let repo_url = EXAMPLE_REPO_URL.to_string();
    let branch = "develop".to_string();
    let cache_path = "/tmp/cache/repo2".to_string();
    let commit_hash = "def456".to_string();

    let cached_repo = CachedRepository::new(repo_url, branch, cache_path, commit_hash);

    assert_eq!(cached_repo.url, EXAMPLE_REPO_URL);
    assert_eq!(cached_repo.branch, "develop");
    assert_eq!(cached_repo.local_cache_path, "/tmp/cache/repo2");
    assert_eq!(cached_repo.commit_hash, "def456");
}
