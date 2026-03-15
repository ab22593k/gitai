//! File status edge case tests
//!
//! Tests for symlink handling, permission changes, special paths, etc.
//!
//! Oracle: **F**amiliarity - compare with standard git behavior
//! Oracle: **W**orld - files behave according to real-world expectations

use git2::Repository;
use gitai::config::Config;
use gitai::core::context::ChangeType;
use std::fs;
use std::path::Path;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{GitTestHelper, setup_git_repo};

#[tokio::test]
/// FILE STATUS: Symlinks should be detected as Added/Modified
/// Oracle: Familiarity - git tracks symlinks as files
#[cfg(unix)]
async fn test_symlink_handling() {
    use std::os::unix::fs::symlink;

    let (temp_dir, git_repo) = setup_git_repo();
    let _helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");

    // Create a target file
    let target_path = temp_dir.path().join("target.txt");
    fs::write(&target_path, "target content").expect("Failed to write target");

    // Create a symlink
    let symlink_path = temp_dir.path().join("link.txt");
    symlink(&target_path, &symlink_path).expect("Failed to create symlink");

    // Stage the symlink
    let mut index = repo.index().expect("Failed to get index");
    index
        .add_path(Path::new("link.txt"))
        .expect("Failed to add symlink");
    index.write().expect("Failed to write index");

    // Get git info
    let config = Config::default();
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // Symlink should be detected
    let symlink_file = context.staged_files.iter().find(|f| f.path == "link.txt");
    assert!(
        symlink_file.is_some(),
        "Symlink should be detected as staged file"
    );

    let file = symlink_file.expect("File should be detected");
    assert!(
        matches!(file.change_type, ChangeType::Added),
        "Symlink should be marked as Added"
    );
}

#[tokio::test]
/// FILE STATUS: Permission changes should be detected
/// Oracle: World - file mode is part of file state
#[cfg(unix)]
async fn test_permission_changes() {
    use std::os::unix::fs::PermissionsExt;

    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");

    // Create a file with normal permissions
    let file_path = temp_dir.path().join("script.sh");
    fs::write(&file_path, "#!/bin/bash\necho hello").expect("Failed to write script");

    // Stage it
    helper
        .create_and_stage_file("script.sh", "#!/bin/bash\necho hello")
        .expect("Failed to stage");

    // Commit it
    helper.commit("Add script").expect("Failed to commit");

    // Change permissions
    let mut perms = fs::metadata(&file_path)
        .expect("Failed to get metadata")
        .permissions();
    perms.set_mode(0o755); // Make executable
    fs::set_permissions(&file_path, perms).expect("Failed to set permissions");

    // Stage the permission change
    let mut index = repo.index().expect("Failed to get index");
    index
        .add_path(Path::new("script.sh"))
        .expect("Failed to add to index");
    index.write().expect("Failed to write index");

    // Get git info
    let config = Config::default();
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // Should detect the change
    let script_file = context.staged_files.iter().find(|f| f.path == "script.sh");
    assert!(
        script_file.is_some(),
        "Permission change should be detected"
    );
}

#[tokio::test]
/// FILE STATUS: Unicode paths should be handled correctly
/// Oracle: World - filenames can contain non-ASCII characters
async fn test_unicode_paths() {
    let (temp_dir, git_repo) = setup_git_repo();
    let _helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create files with unicode names
    let unicode_names = vec![
        "файл.txt",     // Russian
        "文件.txt",     // Chinese
        "ファイル.txt", // Japanese
        "αρχείο.txt",   // Greek
        "file_😀.txt",  // Emoji
    ];

    for name in &unicode_names {
        let file_path = temp_dir.path().join(name);
        fs::write(&file_path, format!("Content of {name}"))
            .unwrap_or_else(|_| panic!("Failed to write unicode file: {name}"));
    }

    // Stage all files
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");
    let mut index = repo.index().expect("Failed to get index");

    for name in &unicode_names {
        index
            .add_path(Path::new(name))
            .expect("Failed to add unicode file");
    }
    index.write().expect("Failed to write index");

    // Get git info
    let config = Config::default();
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // All unicode files should be detected
    for name in &unicode_names {
        let file = context.staged_files.iter().find(|f| f.path == *name);
        assert!(file.is_some(), "Unicode file should be detected: {name}");

        let file = file.expect("File should be detected");
        assert!(
            matches!(file.change_type, ChangeType::Added),
            "Unicode file should be marked as Added: {name}"
        );
    }
}

#[tokio::test]
/// FILE STATUS: Very long paths should be handled
/// Oracle: World - paths can be long but within `PATH_MAX`
async fn test_very_long_paths() {
    let (temp_dir, git_repo) = setup_git_repo();
    let _helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create a deeply nested path
    let mut current_path = temp_dir.path().to_path_buf();
    for i in 0..10 {
        current_path = current_path.join(format!("level_{i}"));
        fs::create_dir_all(&current_path).expect("Failed to create directory");
    }

    // Create a file with a long name at the deepest level
    let long_name = "a".repeat(200); // 200 character filename
    let file_path = current_path.join(format!("{long_name}.txt"));
    fs::write(&file_path, "long path content").expect("Failed to write long path file");

    // Stage the file
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");
    let mut index = repo.index().expect("Failed to get index");

    // Get relative path from repo root
    let relative_path = file_path
        .strip_prefix(temp_dir.path())
        .expect("Failed to get relative path");
    index
        .add_path(relative_path)
        .expect("Failed to add long path file");
    index.write().expect("Failed to write index");

    // Get git info
    let config = Config::default();
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // Long path file should be detected
    let long_file = context
        .staged_files
        .iter()
        .find(|f| f.path.ends_with(&long_name) || f.path.contains("level_9"));

    assert!(long_file.is_some(), "Long path file should be detected");
}

#[tokio::test]
/// FILE STATUS: Empty files should be handled
/// Oracle: World - zero-byte files are valid files
async fn test_empty_file() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create an empty file
    let file_path = temp_dir.path().join("empty.txt");
    fs::write(&file_path, "").expect("Failed to create empty file");

    // Stage it
    helper
        .create_and_stage_file("empty.txt", "")
        .expect("Failed to stage empty file");

    // Get git info
    let config = Config::default();
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // Empty file should be detected
    let empty_file = context.staged_files.iter().find(|f| f.path == "empty.txt");
    assert!(empty_file.is_some(), "Empty file should be detected");

    let file = empty_file.expect("File should be detected");
    assert!(
        matches!(file.change_type, ChangeType::Added),
        "Empty file should be marked as Added"
    );
}

#[tokio::test]
/// FILE STATUS: Directory vs file ambiguity
/// Oracle: World - paths can be either files or directories
async fn test_directory_vs_file_ambiguity() {
    let (temp_dir, git_repo) = setup_git_repo();
    let _helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create a directory
    let dir_path = temp_dir.path().join("ambiguous");
    fs::create_dir_all(&dir_path).expect("Failed to create directory");

    // Git should not track directories themselves, only files within them
    let config = Config::default();
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // Directory itself should not appear in staged files
    let dir_in_staged = context.staged_files.iter().any(|f| f.path == "ambiguous");
    assert!(!dir_in_staged, "Directories should not be tracked by git");
}

#[tokio::test]
/// FILE STATUS: File with spaces in name
/// Oracle: Familiarity - filenames can contain spaces
async fn test_file_with_spaces() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create file with spaces
    let file_name = "file with multiple spaces.txt";
    helper
        .create_and_stage_file(file_name, "content with spaces")
        .expect("Failed to create file with spaces");

    // Get git info
    let config = Config::default();
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // File with spaces should be detected
    let space_file = context.staged_files.iter().find(|f| f.path == file_name);
    assert!(space_file.is_some(), "File with spaces should be detected");

    let file = space_file.expect("File should be detected");
    assert!(
        matches!(file.change_type, ChangeType::Added),
        "File with spaces should be marked as Added"
    );
}
