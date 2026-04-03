#[test]
fn test_normalize_github_url_browser_url_with_tree() {
    let url = "https://github.com/OCA/server-tools/tree/19.0/database_cleanup";
    let normalized = gitai::sync::common::normalize_github_url(url);
    assert_eq!(normalized, "https://github.com/OCA/server-tools.git");
}

#[test]
fn test_normalize_github_url_browser_url_with_blob() {
    let url = "https://github.com/OCA/server-tools/blob/main/src/lib.rs";
    let normalized = gitai::sync::common::normalize_github_url(url);
    assert_eq!(normalized, "https://github.com/OCA/server-tools.git");
}

#[test]
fn test_normalize_github_url_already_git_url() {
    let url = "https://github.com/OCA/server-tools.git";
    let normalized = gitai::sync::common::normalize_github_url(url);
    assert_eq!(normalized, "https://github.com/OCA/server-tools.git");
}

#[test]
fn test_normalize_github_url_bare_repo_url() {
    let url = "https://github.com/OCA/server-tools";
    let normalized = gitai::sync::common::normalize_github_url(url);
    assert_eq!(normalized, "https://github.com/OCA/server-tools.git");
}

#[test]
fn test_normalize_github_url_non_github() {
    let url = "https://gitlab.com/example/repo.git";
    let normalized = gitai::sync::common::normalize_github_url(url);
    assert_eq!(normalized, "https://gitlab.com/example/repo.git");
}

#[test]
fn test_normalize_github_url_with_trailing_slash() {
    let url = "https://github.com/OCA/server-tools/";
    let normalized = gitai::sync::common::normalize_github_url(url);
    assert_eq!(normalized, "https://github.com/OCA/server-tools.git");
}
