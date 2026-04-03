use git2::Repository;
use gitai::{config::Config, git::GitRepo};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn setup_git_repo() -> (TempDir, GitRepo) {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let repo = Repository::init(temp_dir.path()).expect("Failed to initialize repository");

    // Configure git user
    let mut config = repo.config().expect("Failed to get repository config");
    config
        .set_str("user.name", "Test User")
        .expect("Failed to set user name");
    config
        .set_str("user.email", "test@example.com")
        .expect("Failed to set user email");

    // Create and commit an initial file
    let initial_file_path = temp_dir.path().join("initial.txt");
    fs::write(&initial_file_path, "Initial content").expect("Failed to write initial file");

    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("initial.txt"))
        .expect("Failed to add file to index");
    index.write().expect("Failed to write index");

    let tree_id = index.write_tree().expect("Failed to write tree");
    let tree = repo.find_tree(tree_id).expect("Failed to find tree");
    let signature = repo.signature().expect("Failed to create signature");
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )
    .expect("Failed to commit");

    // Ensure the default branch is named 'main' for consistency across environments
    {
        let head_commit = repo
            .head()
            .expect("Failed to get HEAD")
            .peel_to_commit()
            .expect("Failed to peel HEAD to commit");
        let current_branch = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(std::string::ToString::to_string))
            .unwrap_or_default();
        if current_branch != "main" {
            // Create or update the 'main' branch pointing to the current HEAD commit
            repo.branch("main", &head_commit, true)
                .expect("Failed to create 'main' branch");
            repo.set_head("refs/heads/main")
                .expect("Failed to set HEAD to 'main' branch");
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .expect("Failed to checkout 'main' branch");
        }
    }

    let git_repo = GitRepo::new(temp_dir.path()).expect("Failed to create GitRepo");
    (temp_dir, git_repo)
}

#[tokio::test]
async fn test_get_git_info_with_excluded_files() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Create files that should be excluded
    fs::create_dir_all(temp_dir.path().join("node_modules"))
        .expect("Failed to create node_modules directory");
    fs::write(
        temp_dir.path().join("node_modules/excluded.js"),
        "console.log('excluded');",
    )
    .expect("Failed to write excluded file");
    fs::write(
        temp_dir.path().join(".gitignore"),
        "node_modules/\npackage-lock.json",
    )
    .expect("Failed to write .gitignore");
    fs::write(
        temp_dir.path().join("package-lock.json"),
        r#"{"name": "test-package"}"#,
    )
    .expect("Failed to write package-lock.json");

    // Create a non-excluded file
    fs::write(
        temp_dir.path().join("included.js"),
        "console.log('included');",
    )
    .expect("Failed to write included file");

    // Stage all files
    let repo = Repository::open(temp_dir.path()).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");

    // Force add all files, including ignored ones for testing
    index
        .add_all(["*"].iter(), git2::IndexAddOption::FORCE, None)
        .expect("Failed to add all files to index");
    index.write().expect("Failed to write index");

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // Check excluded files
    let excluded_files: Vec<_> = context
        .staged_files
        .iter()
        .filter(|file| file.content_excluded)
        .collect();

    assert!(!excluded_files.is_empty(), "Should have excluded files");

    assert!(
        excluded_files
            .iter()
            .any(|file| file.path == "package-lock.json")
    );

    for file in &excluded_files {
        assert_eq!(file.diff, "[Content excluded]");
    }

    // Check included file
    let included_files: Vec<_> = context
        .staged_files
        .iter()
        .filter(|file| !file.content_excluded)
        .collect();

    assert!(!included_files.is_empty(), "Should have included files");
    assert!(included_files.iter().any(|file| file.path == "included.js"));

    for file in &included_files {
        assert_ne!(file.diff, "[Content excluded]");
    }
}

#[tokio::test]
async fn test_multiple_staged_files_with_exclusions() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Create .gitignore with exclusion patterns
    fs::write(temp_dir.path().join(".gitignore"), ".vscode/\n*.min.js")
        .expect("Failed to write .gitignore");

    // Create files that should be excluded
    fs::create_dir_all(temp_dir.path().join(".vscode"))
        .expect("Failed to create .vscode directory");
    fs::write(
        temp_dir.path().join(".vscode/settings.json"),
        r#"{"editor.formatOnSave": true}"#,
    )
    .expect("Failed to write .vscode/settings.json");
    fs::write(
        temp_dir.path().join("large.min.js"),
        "console.log('minified')",
    )
    .expect("Failed to write large.min.js");

    // Create non-excluded files
    for i in 1..=3 {
        fs::write(
            temp_dir.path().join(format!("file{i}.txt")),
            format!("Content {i}"),
        )
        .expect("Failed to write non-excluded file");
    }

    // Stage all files
    let repo = Repository::open(temp_dir.path()).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");

    // Force add all files, including ignored ones for testing
    index
        .add_all(["*"].iter(), git2::IndexAddOption::FORCE, None)
        .expect("Failed to add all files to index");
    index.write().expect("Failed to write index");

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    assert_eq!(context.staged_files.len(), 6); // 3 files + .gitignore

    let excluded_files: Vec<_> = context
        .staged_files
        .iter()
        .filter(|file| file.content_excluded)
        .collect();
    assert_eq!(excluded_files.len(), 2);

    let included_files: Vec<_> = context
        .staged_files
        .iter()
        .filter(|file| !file.content_excluded)
        .collect();
    assert_eq!(included_files.len(), 4); // 3 files + .gitignore

    for file in &excluded_files {
        assert!(file.path.contains(".vscode") || file.path.contains(".min.js"));
        assert_eq!(file.diff, "[Content excluded]");
    }

    for file in &included_files {
        assert!(
            (file.path.starts_with("file")
                && std::path::Path::new(&file.path)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("txt")))
                || file.path == ".gitignore"
        );
        assert_ne!(file.diff, "[Content excluded]");
    }
}

#[tokio::test]
async fn test_nested_gitignore_files() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Create nested directory structure
    fs::create_dir_all(temp_dir.path().join("src")).expect("Failed to create src directory");
    fs::create_dir_all(temp_dir.path().join("src/subdir")).expect("Failed to create src/subdir");

    // Create .gitignore in root
    fs::write(temp_dir.path().join(".gitignore"), "*.log\ntest.txt\n")
        .expect("Failed to write root .gitignore");

    // Create .gitignore in subdirectory
    fs::write(temp_dir.path().join("src/.gitignore"), "temp.*\n")
        .expect("Failed to write src/.gitignore");

    // Create files that should be excluded by different gitignores
    fs::write(temp_dir.path().join("debug.log"), "debug info").expect("Failed to write debug.log");
    fs::write(temp_dir.path().join("test.txt"), "test content").expect("Failed to write test.txt");
    fs::write(temp_dir.path().join("src/temp.dat"), "temp data")
        .expect("Failed to write src/temp.dat");
    fs::write(temp_dir.path().join("src/subdir/temp.txt"), "nested temp")
        .expect("Failed to write src/subdir/temp.txt");

    // Create included file
    fs::write(temp_dir.path().join("src/important.rs"), "fn main() {}")
        .expect("Failed to write important.rs");

    // Stage all files
    let repo = Repository::open(temp_dir.path()).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_all(["*"].iter(), git2::IndexAddOption::FORCE, None)
        .expect("Failed to add all files to index");
    index.write().expect("Failed to write index");

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // Check that files are properly excluded/included
    let excluded_files: Vec<_> = context
        .staged_files
        .iter()
        .filter(|file| file.content_excluded)
        .map(|f| f.path.as_str())
        .collect();

    let included_files: Vec<_> = context
        .staged_files
        .iter()
        .filter(|file| !file.content_excluded)
        .map(|f| f.path.as_str())
        .collect();

    // debug.log and test.txt should be excluded by root .gitignore
    assert!(excluded_files.contains(&"debug.log"));
    assert!(excluded_files.contains(&"test.txt"));

    // src/temp.dat should be excluded by src/.gitignore
    assert!(excluded_files.contains(&"src/temp.dat"));

    // src/subdir/temp.txt should be excluded by src/.gitignore (temp.* pattern)
    assert!(excluded_files.contains(&"src/subdir/temp.txt"));

    // src/important.rs should be included
    assert!(included_files.contains(&"src/important.rs"));
}
