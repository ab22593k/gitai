#![allow(clippy::unwrap_used, clippy::needless_raw_string_hashes)]

use gitai::sync::common::Parsed;
use gitai::sync::common::parse::parse_gitwire;
use gitai::sync::common::parse::save_to_gitwire;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn get_gitwire_path(base_dir: &Path, _global: bool) -> std::path::PathBuf {
    base_dir.join(".gitwire")
}

#[test]
fn test_parse_gitwire_local_file_not_exists() {
    let temp_dir = TempDir::new().unwrap();
    let repo_dir = temp_dir.path();

    let result = parse_gitwire(repo_dir, false);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_parse_gitwire_global_file_not_exists() {
    let temp_dir = TempDir::new().unwrap();
    let home_dir = temp_dir.path();

    let result = parse_gitwire(home_dir, true);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_parse_gitwire_local_file_exists() {
    let temp_dir = TempDir::new().unwrap();
    let repo_dir = temp_dir.path();

    let gitwire_content = r#"
[wire]
    url = https://github.com/foo/bar
    dst = libs/bar
    rev = main
"#;

    let gitwire_path = get_gitwire_path(repo_dir, false);
    fs::write(&gitwire_path, gitwire_content).unwrap();

    let result = parse_gitwire(repo_dir, false);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert!(parsed.is_some());

    let entries = parsed.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].url, "https://github.com/foo/bar");
    assert_eq!(entries[0].dst, "libs/bar");
    assert_eq!(entries[0].rev, "main");
}

#[test]
fn test_parse_gitwire_global_file_exists() {}

#[test]
fn test_parse_gitwire_multiple_entries() {
    let temp_dir = TempDir::new().unwrap();
    let repo_dir = temp_dir.path();

    let gitwire_content = r#"
[wire "libs/bar"]
    url = https://github.com/foo/bar
    rev = main

[wire "libs/baz"]
    url = https://github.com/baz/qux
    rev = v2.0.0
"#;

    let gitwire_path = get_gitwire_path(repo_dir, false);
    fs::write(&gitwire_path, gitwire_content).unwrap();

    let result = parse_gitwire(repo_dir, false);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert!(parsed.is_some());

    let entries = parsed.unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn test_save_and_parse_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let repo_dir = temp_dir.path();

    let entry = Parsed {
        name: None,
        dsc: None,
        url: "https://github.com/test/repo".to_string(),
        rev: "main".to_string(),
        src: vec!["src/".to_string()],
        dst: "libs/test".to_string(),
        mtd: None,
        last_sync_hash: None,
        merge_strategy: None,
    };

    let save_result = save_to_gitwire(repo_dir, false, &entry);
    assert!(save_result.is_ok());

    let parsed_result = parse_gitwire(repo_dir, false);
    assert!(parsed_result.is_ok());

    let parsed = parsed_result.unwrap();
    assert!(parsed.is_some());

    let entries = parsed.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].url, "https://github.com/test/repo");
    assert_eq!(entries[0].dst, "libs/test");
    assert_eq!(entries[0].rev, "main");
}

#[test]
fn test_parse_gitwire_with_name() {
    let temp_dir = TempDir::new().unwrap();
    let repo_dir = temp_dir.path();

    let gitwire_content = r#"
[wire "myrepo"]
    url = https://github.com/foo/bar
    dst = libs/bar
    rev = main
"#;

    let gitwire_path = get_gitwire_path(repo_dir, false);
    fs::write(&gitwire_path, gitwire_content).unwrap();

    let result = parse_gitwire(repo_dir, false);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert!(parsed.is_some());

    let entries = parsed.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, Some("myrepo".to_string()));
}

#[test]
fn test_parse_gitwire_with_description() {
    let temp_dir = TempDir::new().unwrap();
    let repo_dir = temp_dir.path();

    let gitwire_content = r#"
[wire "myrepo"]
    url = https://github.com/foo/bar
    dst = libs/bar
    rev = main
    description = My awesome library
"#;

    let gitwire_path = get_gitwire_path(repo_dir, false);
    fs::write(&gitwire_path, gitwire_content).unwrap();

    let result = parse_gitwire(repo_dir, false);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    let entries = parsed.unwrap();
    assert_eq!(entries[0].dsc, Some("My awesome library".to_string()));
}

#[test]
fn test_parse_gitwire_with_method() {
    let temp_dir = TempDir::new().unwrap();
    let repo_dir = temp_dir.path();

    let gitwire_content = r#"
[wire "myrepo"]
    url = https://github.com/foo/bar
    dst = libs/bar
    rev = main
    method = shallow
"#;

    let gitwire_path = get_gitwire_path(repo_dir, false);
    fs::write(&gitwire_path, gitwire_content).unwrap();

    let result = parse_gitwire(repo_dir, false);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    let entries = parsed.unwrap();
    assert!(entries[0].mtd.is_some());
}
