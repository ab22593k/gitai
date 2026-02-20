use git2::Repository;
use gitai::core::context::ChangeType;
use gitai::git::GitRepo;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{GitTestHelper, MockDataBuilder, TestAssertions, setup_git_repo};

#[tokio::test]
async fn test_get_git_info() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = MockDataBuilder::config();

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // Use centralized assertions
    TestAssertions::assert_commit_context_basics(&context);

    // Test branch name
    assert!(
        context.branch == "main" || context.branch == "master",
        "Branch should be 'main' or 'master', but got '{}'",
        context.branch
    );

    // Test recent commits
    assert_eq!(context.recent_commits.len(), 1);
    assert!(context.recent_commits[0].message.contains("Initial commit"));

    // Test staged files (should be empty after commit)
    assert_eq!(context.staged_files.len(), 0);

    // Create and stage a new file using helper
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create GitTestHelper");
    helper
        .create_and_stage_file("new_file.txt", "New content")
        .expect("Failed to create and stage file");

    // Create an unstaged file
    let unstaged_file_path = temp_dir.path().join("unstaged.txt");
    fs::write(&unstaged_file_path, "Unstaged content").expect("Failed to write unstaged file");

    // Get updated git info
    let updated_context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get updated git info");

    // Test staged files
    assert_eq!(updated_context.staged_files.len(), 1);
    assert_eq!(updated_context.staged_files[0].path, "new_file.txt");
    assert!(matches!(
        updated_context.staged_files[0].change_type,
        ChangeType::Added
    ));
}

#[tokio::test]
async fn test_commit() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = MockDataBuilder::config();

    // Create and stage a new file using helper
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create GitTestHelper");
    helper
        .create_and_stage_file("commit_test.txt", "Commit test content")
        .expect("Failed to create and stage file");

    // Perform commit
    let result = git_repo.commit("Test commit message");
    assert!(result.is_ok(), "Failed to perform commit");

    // Verify commit
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info after commit");
    assert_eq!(context.recent_commits.len(), 2);
    assert!(
        context.recent_commits[0]
            .message
            .contains("Test commit message")
    );
}

#[tokio::test]
async fn test_multiple_staged_files() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = MockDataBuilder::config();

    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create GitTestHelper");

    // Create and stage multiple files using helper
    for i in 1..=3 {
        helper
            .create_and_stage_file(&format!("file{i}.txt"), &format!("Content {i}"))
            .expect("Failed to create and stage file");
    }

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    assert_eq!(context.staged_files.len(), 3);
    for i in 1..=3 {
        assert!(
            context
                .staged_files
                .iter()
                .any(|file| file.path == format!("file{i}.txt"))
        );
    }
}

#[tokio::test]
async fn test_modified_file() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = MockDataBuilder::config();

    // Modify the initial file and stage it using helper
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create GitTestHelper");
    helper
        .create_and_stage_file("initial.txt", "Modified content")
        .expect("Failed to modify and stage file");

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    assert_eq!(context.staged_files.len(), 1);
    assert!(
        context
            .staged_files
            .iter()
            .any(|file| file.path == "initial.txt"
                && matches!(file.change_type, ChangeType::Modified))
    );
}

#[tokio::test]
async fn test_deleted_file() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = MockDataBuilder::config();

    // Delete the initial file
    let initial_file_path = temp_dir.path().join("initial.txt");
    fs::remove_file(&initial_file_path).expect("Failed to remove initial file");

    // Stage the deletion
    let repo = Repository::open(temp_dir.path()).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");
    index
        .remove_path(Path::new("initial.txt"))
        .expect("Failed to remove file from index");
    index.write().expect("Failed to write index");

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    assert_eq!(context.staged_files.len(), 1);
    assert!(
        context
            .staged_files
            .iter()
            .any(|file| file.path == "initial.txt"
                && matches!(file.change_type, ChangeType::Deleted))
    );
}

#[tokio::test]
async fn test_binary_file() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = MockDataBuilder::config();

    // Create a binary file using mock data
    let binary_content = MockDataBuilder::mock_binary_content();
    let binary_file_path = temp_dir.path().join("image.png");
    fs::write(&binary_file_path, binary_content).expect("Failed to write binary file");

    // Stage the binary file (need to use git2 directly for existing files)
    let repo = Repository::open(temp_dir.path()).expect("Failed to open repository");
    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("image.png"))
        .expect("Failed to add binary file to index");
    index.write().expect("Failed to write index");

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    // Check if the binary file is in staged files
    assert!(
        context
            .staged_files
            .iter()
            .any(|file| file.path == "image.png")
    );

    // Check if the diff for the binary file is "[Binary file changed]"
    let binary_file = context
        .staged_files
        .iter()
        .find(|file| file.path == "image.png")
        .expect("Failed to find binary file in staged files");
    assert_eq!(binary_file.diff, "[Binary file changed]");

    // Check if the status is correct
    assert!(matches!(binary_file.change_type, ChangeType::Added));
}

#[tokio::test]
async fn test_fresh_repo_commit_generation() {
    // Create a fresh git repo without any commits
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let repo = Repository::init(temp_dir.path()).expect("Failed to initialize repository");

    // Configure git user (required for commits)
    let mut config = repo.config().expect("Failed to get repository config");
    config
        .set_str("user.name", "Test User")
        .expect("Failed to set user name");
    config
        .set_str("user.email", "test@example.com")
        .expect("Failed to set user email");

    // Create and stage a file
    let initial_file_path = temp_dir.path().join("README.md");
    fs::write(&initial_file_path, "# Fresh Project").expect("Failed to write initial file");

    let mut index = repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("README.md"))
        .expect("Failed to add file to index");
    index.write().expect("Failed to write index");

    let git_repo = GitRepo::new(temp_dir.path()).expect("Failed to create GitRepo");
    let config = MockDataBuilder::config();

    // This should work even on a fresh repo with no commits
    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info for fresh repo");

    // Verify that recent_commits is empty (no commits yet)
    assert_eq!(context.recent_commits.len(), 0);

    // Verify that author_history is empty
    assert_eq!(context.author_history.len(), 0);

    // Verify that staged files are present
    assert_eq!(context.staged_files.len(), 1);
    assert_eq!(context.staged_files[0].path, "README.md");
    assert!(matches!(
        context.staged_files[0].change_type,
        ChangeType::Added
    ));
}
