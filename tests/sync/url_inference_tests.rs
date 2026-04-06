#[test]
fn test_infer_from_url_tree_path() {
    let url = "https://github.com/OCA/server-tools/tree/19.0/base_technical_user";
    let result = gitai::sync::common::infer_from_url(url);
    let (rev, src) = result.expect("Should extract rev and src from tree URL");
    assert_eq!(rev, "19.0");
    assert_eq!(src, vec!["base_technical_user"]);
}

#[test]
fn test_infer_from_url_blob_path() {
    let url = "https://github.com/OCA/server-tools/blob/19.0/base_technical_user/src/lib.rs";
    let result = gitai::sync::common::infer_from_url(url);
    let (rev, src) = result.expect("Should extract rev and src from blob URL");
    assert_eq!(rev, "19.0");
    assert_eq!(src, vec!["base_technical_user/src"]);
}

#[test]
fn test_infer_from_url_nested_path() {
    let url = "https://github.com/OCA/server-tools/tree/main/subdir/module";
    let result = gitai::sync::common::infer_from_url(url);
    let (rev, src) = result.expect("Should extract rev and src from nested tree URL");
    assert_eq!(rev, "main");
    assert_eq!(src, vec!["subdir/module"]);
}

#[test]
fn test_infer_from_url_non_github() {
    let url = "https://gitlab.com/example/repo.git";
    let result = gitai::sync::common::infer_from_url(url);
    assert!(result.is_none(), "Non-GitHub URLs should return None");
}

#[test]
fn test_infer_from_url_bare_repo() {
    let url = "https://github.com/OCA/server-tools";
    let result = gitai::sync::common::infer_from_url(url);
    assert!(result.is_none(), "Bare repo URL should return None");
}

#[test]
fn test_infer_from_url_already_git_url() {
    let url = "https://github.com/OCA/server-tools.git";
    let result = gitai::sync::common::infer_from_url(url);
    assert!(result.is_none(), "Already-git URL should return None");
}

#[test]
fn test_infer_from_url_branch_with_hyphen() {
    let url = "https://github.com/example/repo/tree/release-1.0/frontend";
    let result = gitai::sync::common::infer_from_url(url);
    let (rev, src) = result.expect("Should extract rev with hyphen");
    assert_eq!(rev, "release-1.0");
    assert_eq!(src, vec!["frontend"]);
}
