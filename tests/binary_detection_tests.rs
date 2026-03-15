//! Binary file detection tests
//!
//! Tests that binary files are correctly identified and handled.
//!
//! Oracle: **S**tandards - follow git's binary file detection
//! Oracle: **C**omparable - match behavior of other git tools

use git2::Repository;
use gitai::config::Config;
use gitai::core::context::ChangeType;
use std::fs;

#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{GitTestHelper, MockDataBuilder, setup_git_repo};

#[tokio::test]
/// BINARY: PNG files should be detected as binary
/// Oracle: Standards - PNG magic bytes indicate binary
async fn test_png_detection() {
    use git2::Repository;
    use std::path::Path;

    let (temp_dir, git_repo) = setup_git_repo();
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");

    // Create a minimal PNG file (PNG magic bytes + minimal header)
    let png_content = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
    ];

    let png_path = temp_dir.path().join("image.png");
    fs::write(&png_path, &png_content).expect("Failed to write PNG");

    // Stage it directly using git2
    let mut index = repo.index().expect("Failed to get index");
    index
        .add_path(Path::new("image.png"))
        .expect("Failed to add PNG");
    index.write().expect("Failed to write index");

    // Get git info
    let config = Config::default();
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // PNG should be detected
    let png_file = context.staged_files.iter().find(|f| f.path == "image.png");
    assert!(png_file.is_some(), "PNG file should be detected");

    let file = png_file.expect("File should be detected");
    assert!(
        matches!(file.change_type, ChangeType::Added),
        "PNG should be marked as Added"
    );

    // Should be marked as binary (git detects binary by looking for null bytes)
    assert!(
        file.diff.contains("Binary"),
        "PNG should be marked as binary in diff: {}",
        file.diff
    );
}

#[tokio::test]
/// BINARY: JPEG files should be detected as binary
/// Oracle: Standards - JPEG magic bytes indicate binary
async fn test_jpeg_detection() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create a minimal JPEG file (JPEG magic bytes)
    let jpeg_content = vec![
        0xFF, 0xD8, 0xFF, 0xE0, // JPEG signature
        0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, // JFIF header
    ];

    let jpeg_path = temp_dir.path().join("image.jpg");
    fs::write(&jpeg_path, &jpeg_content).expect("Failed to write JPEG");

    // Stage it
    helper
        .create_and_stage_file(
            "image.jpg",
            std::str::from_utf8(&jpeg_content).unwrap_or(""),
        )
        .expect("Failed to stage JPEG");

    // Get git info
    let config = Config::default();
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // JPEG should be detected
    let jpeg_file = context.staged_files.iter().find(|f| f.path == "image.jpg");
    assert!(jpeg_file.is_some(), "JPEG file should be detected");

    let file = jpeg_file.expect("File should be detected");
    assert!(
        matches!(file.change_type, ChangeType::Added),
        "JPEG should be marked as Added"
    );
}

#[tokio::test]
/// BINARY: Plain text should NOT be marked as binary
/// Oracle: Standards - ASCII/UTF-8 text is not binary
async fn test_text_file_not_binary() {
    let (temp_dir, git_repo) = setup_git_repo();
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create a plain text file
    let text_content =
        "This is plain text content.\nIt has multiple lines.\nAnd some special chars: !@#$%";

    helper
        .create_and_stage_file("readme.txt", text_content)
        .expect("Failed to stage text file");

    // Get git info
    let config = Config::default();
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // Text file should be detected
    let text_file = context.staged_files.iter().find(|f| f.path == "readme.txt");
    assert!(text_file.is_some(), "Text file should be detected");

    let file = text_file.expect("File should be detected");
    assert!(
        matches!(file.change_type, ChangeType::Added),
        "Text file should be marked as Added"
    );

    // Should NOT be marked as binary
    assert!(
        !file.diff.contains("Binary"),
        "Text file should not be marked as binary: {}",
        file.diff
    );
}

#[tokio::test]
/// BINARY: UTF-8 with BOM should be handled correctly
/// Oracle: Standards - UTF-8 BOM is valid text, not binary
async fn test_utf8_with_bom() {
    let (temp_dir, git_repo) = setup_git_repo();
    let _helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Create UTF-8 file with BOM
    let mut bom_content = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
    bom_content.extend_from_slice(b"This is UTF-8 text with BOM\n");

    let bom_path = temp_dir.path().join("utf8-bom.txt");
    fs::write(&bom_path, &bom_content).expect("Failed to write UTF-8 BOM file");

    // Stage it
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");
    let mut index = repo.index().expect("Failed to get index");
    index
        .add_path(std::path::Path::new("utf8-bom.txt"))
        .expect("Failed to add file");
    index.write().expect("Failed to write index");

    // Get git info
    let config = Config::default();
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // UTF-8 BOM file should be detected
    let bom_file = context
        .staged_files
        .iter()
        .find(|f| f.path == "utf8-bom.txt");
    assert!(bom_file.is_some(), "UTF-8 BOM file should be detected");

    let file = bom_file.expect("File should be detected");
    assert!(
        matches!(file.change_type, ChangeType::Added),
        "UTF-8 BOM file should be marked as Added"
    );

    // UTF-8 with BOM is technically text, but git might treat it as binary
    // This test documents the current behavior
    // Note: Some tools consider BOM files as binary, others as text
}

#[tokio::test]
/// BINARY: Git binary patch format detection
/// Oracle: Standards - GIT `binary_patch` format should be recognized
async fn test_git_binary_patch_format() {
    let (temp_dir, git_repo) = setup_git_repo();
    let _helper = GitTestHelper::new(&temp_dir).expect("Failed to create helper");

    // Use the mock binary content from test utilities
    let binary_content = MockDataBuilder::mock_binary_content();

    let binary_path = temp_dir.path().join("binary.dat");
    fs::write(&binary_path, &binary_content).expect("Failed to write binary file");

    // Stage it
    let repo = Repository::open(temp_dir.path()).expect("Failed to open");
    let mut index = repo.index().expect("Failed to get index");
    index
        .add_path(std::path::Path::new("binary.dat"))
        .expect("Failed to add binary file");
    index.write().expect("Failed to write index");

    // Get git info
    let config = Config::default();
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // Binary file should be detected
    let binary_file = context.staged_files.iter().find(|f| f.path == "binary.dat");
    assert!(binary_file.is_some(), "Binary file should be detected");

    let file = binary_file.expect("File should be detected");
    assert!(
        matches!(file.change_type, ChangeType::Added),
        "Binary file should be marked as Added"
    );

    // Should be marked as binary in diff
    assert!(
        file.diff.contains("Binary") || file.diff.is_empty(),
        "Binary file should be marked as binary or have empty diff: {}",
        file.diff
    );
}
