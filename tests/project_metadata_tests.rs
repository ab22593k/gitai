use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::setup_temp_dir;

#[tokio::test]
async fn test_project_metadata_parallelism() {
    // Create a temporary directory for our test files using our centralized infrastructure
    let (temp_dir, git_repo) = setup_temp_dir();

    // Create multiple files with different "languages"
    let files = vec![
        ("file1.rs", "fn main() {}"),
        ("file2.py", "def main(): pass"),
        ("file3.js", "function main() {}"),
        ("file3.js", "function main() {}"),
        ("file4.c", "int main() { return 0; }"),
        ("file5.kt", "fun main() {}"),
    ];

    let file_paths: Vec<String> = files
        .into_iter()
        .map(|(filename, content)| {
            let file_path = temp_dir.path().join(filename);
            fs::write(&file_path, content).expect("Failed to write test file");
            let path_str = file_path
                .to_str()
                .expect("Failed to convert path to string")
                .to_string();
            println!("Created file: {path_str} with content: {content}");
            assert!(
                Path::new(&path_str).exists(),
                "File does not exist: {path_str}"
            );
            path_str
        })
        .collect();

    // Measure the time taken to process metadata
    let start = Instant::now();
    let metadata = git_repo
        .get_project_metadata(&file_paths)
        .await
        .expect("Failed to get project metadata");
    let duration = start.elapsed();

    // Detailed logging
    println!("File paths: {file_paths:?}");
    println!("Metadata: {metadata:?}");
    println!("Detected language: {:?}", metadata.language);
    println!("Detected dependencies: {:?}", metadata.dependencies);
    println!("Processing time: {duration:?}");

    // Assertions
    assert!(metadata.language.is_some(), "Language should be detected");

    let languages = metadata.language.expect("Failed to detect languages");
    assert!(languages.contains("Rust"), "Rust should be detected");
    assert!(languages.contains("Python"), "Python should be detected");
    assert!(
        languages.contains("JavaScript"),
        "JavaScript should be detected"
    );
    assert!(languages.contains('C'), "C should be detected");
    assert!(languages.contains("Kotlin"), "Kotlin should be detected");

    // We're not expecting any dependencies in this test
    assert!(
        metadata.dependencies.is_empty(),
        "No dependencies should be detected"
    );

    // Check if the operation was faster than sequential execution would be
    assert!(
        duration < Duration::from_millis(500),
        "Parallel execution took too long: {duration:?}"
    );
}
