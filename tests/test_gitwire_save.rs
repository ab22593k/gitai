use std::fs;
use tempfile::TempDir;

use gitai::Parsed;
use gitai::sync::common::parse::save_to_gitwire;

fn create_test_parsed(name: &str, url: &str, rev: &str, src: &str, dst: &str) -> Parsed {
    Parsed {
        name: Some(name.to_string()),
        dsc: None,
        url: url.to_string(),
        rev: rev.to_string(),
        src: vec![src.to_string()],
        dst: dst.to_string(),
        mtd: None,
        last_sync_hash: None,
        merge_strategy: None,
    }
}

#[test]
fn test_save_to_gitwire_creates_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let entry = create_test_parsed(
        "test-entry",
        "https://github.com/example/repo.git",
        "main",
        "src/",
        "vendor/src",
    );

    save_to_gitwire(temp_dir.path(), false, &entry, false).expect("Failed to save .gitwire");

    let gitwire_path = temp_dir.path().join(".gitwire");
    assert!(gitwire_path.exists(), ".gitwire file should be created");

    let content = fs::read_to_string(&gitwire_path).expect("Failed to read .gitwire");
    assert!(content.contains("url = https://github.com/example/repo.git"));
    assert!(content.contains("rev = main"));
    assert!(content.contains("src = src/"));
    assert!(content.contains("dst = vendor/src"));
}

#[test]
fn test_save_to_gitwire_overwrites_existing() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let entry1 = create_test_parsed(
        "entry-one",
        "https://github.com/example/repo1.git",
        "main",
        "src/",
        "vendor/src1",
    );
    save_to_gitwire(temp_dir.path(), false, &entry1, false).expect("Failed to save first entry");

    let entry2 = create_test_parsed(
        "entry-two",
        "https://github.com/example/repo2.git",
        "develop",
        "lib/",
        "vendor/src2",
    );
    save_to_gitwire(temp_dir.path(), false, &entry2, false)
        .expect("Failed to save second entry (overwrite)");

    let content =
        fs::read_to_string(temp_dir.path().join(".gitwire")).expect("Failed to read .gitwire");
    assert!(content.contains("repo2.git"));
    assert!(
        !content.contains("repo1.git"),
        "Should not contain old entry"
    );
}

#[test]
fn test_save_to_gitwire_appends_existing() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let entry1 = create_test_parsed(
        "entry-one",
        "https://github.com/example/repo1.git",
        "main",
        "src/",
        "vendor/src1",
    );
    save_to_gitwire(temp_dir.path(), false, &entry1, false).expect("Failed to save first entry");

    let entry2 = create_test_parsed(
        "entry-two",
        "https://github.com/example/repo2.git",
        "develop",
        "lib/",
        "vendor/src2",
    );
    save_to_gitwire(temp_dir.path(), false, &entry2, true).expect("Failed to append second entry");

    let content =
        fs::read_to_string(temp_dir.path().join(".gitwire")).expect("Failed to read .gitwire");
    assert!(content.contains("repo1.git"), "Should contain first entry");
    assert!(content.contains("repo2.git"), "Should contain second entry");
}

#[test]
fn test_save_to_gitwire_append_creates_file_if_not_exists() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let entry = create_test_parsed(
        "test-entry",
        "https://github.com/example/repo.git",
        "main",
        "src/",
        "vendor/src",
    );

    save_to_gitwire(temp_dir.path(), false, &entry, true)
        .expect("Failed to save .gitwire with append");

    let content =
        fs::read_to_string(temp_dir.path().join(".gitwire")).expect("Failed to read .gitwire");
    assert!(content.contains("repo.git"));
}

#[test]
fn test_save_to_gitwire_multiple_entries_parseable() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let entry1 = create_test_parsed(
        "entry-one",
        "https://github.com/example/repo1.git",
        "main",
        "src/",
        "vendor/src1",
    );
    save_to_gitwire(temp_dir.path(), false, &entry1, false).expect("Failed to save first entry");

    let entry2 = create_test_parsed(
        "entry-two",
        "https://github.com/example/repo2.git",
        "develop",
        "lib/",
        "vendor/src2",
    );
    save_to_gitwire(temp_dir.path(), false, &entry2, true).expect("Failed to append second entry");

    let parsed = gitai::parse_gitwire(temp_dir.path(), false)
        .expect("Failed to parse .gitwire")
        .expect("Should have parsed entries");

    assert_eq!(parsed.len(), 2, "Should have two entries");
}
