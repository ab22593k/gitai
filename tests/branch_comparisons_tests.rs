use gitai::core::context::ChangeType;
use std::fs;
use std::path::Path;

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{GitTestHelper, MockDataBuilder, setup_git_repo};

#[tokio::test]
async fn test_branch_comparison_basic() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = MockDataBuilder::config();

    // Use our GitTestHelper for branch operations
    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create GitTestHelper");

    // Create a feature branch
    helper
        .create_branch("feature-branch")
        .expect("Failed to create feature branch");
    helper
        .checkout_branch("feature-branch")
        .expect("Failed to checkout feature branch");

    // Add a new file using our helper
    helper
        .create_and_stage_file("feature.txt", "Feature content")
        .expect("Failed to create and stage feature file");

    // Commit the changes
    helper
        .commit("Add feature file")
        .expect("Failed to commit feature");

    // Test branch comparison
    let context = git_repo
        .get_git_info_for_branch_diff(&config, "main", "feature-branch")
        .expect("Failed to get git info for branch diff");

    // Test branch name formatting
    assert_eq!(context.branch, "main -> feature-branch");

    // Test that we have the feature commit
    assert_eq!(context.recent_commits.len(), 1);
    assert!(
        context.recent_commits[0]
            .message
            .contains("Add feature file")
    );

    // Test staged files (should contain the feature file)
    assert_eq!(context.staged_files.len(), 1);
    assert_eq!(context.staged_files[0].path, "feature.txt");
    assert!(matches!(
        context.staged_files[0].change_type,
        ChangeType::Added
    ));
    assert!(context.staged_files[0].content.is_some());
    assert_eq!(
        context.staged_files[0]
            .content
            .as_ref()
            .expect("Should have content"),
        "Feature content"
    );
}

#[tokio::test]
async fn test_branch_comparison_multiple_files() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = MockDataBuilder::config();

    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create GitTestHelper");

    // Create feature branch and switch to it
    helper
        .create_branch("feature-multiple")
        .expect("Failed to create feature branch");
    helper
        .checkout_branch("feature-multiple")
        .expect("Failed to checkout feature branch");

    // Add multiple files using our helper
    for i in 1..=3 {
        helper
            .create_and_stage_file(&format!("feature{i}.txt"), &format!("Feature {i} content"))
            .expect("Failed to create and stage feature file");
    }

    // Modify the initial file using our helper
    helper
        .create_and_stage_file("initial.txt", "Modified in feature branch")
        .expect("Failed to modify and stage initial file");

    // Commit all changes
    helper
        .commit("Add multiple features and modify initial")
        .expect("Failed to commit multiple features");

    // Test branch comparison
    let context = git_repo
        .get_git_info_for_branch_diff(&config, "main", "feature-multiple")
        .expect("Failed to get git info for branch diff");

    // Should have 4 files: 3 new + 1 modified
    assert_eq!(context.staged_files.len(), 4);

    // Count file types
    let added_files: Vec<_> = context
        .staged_files
        .iter()
        .filter(|f| matches!(f.change_type, ChangeType::Added))
        .collect();
    let modified_files: Vec<_> = context
        .staged_files
        .iter()
        .filter(|f| matches!(f.change_type, ChangeType::Modified))
        .collect();

    assert_eq!(added_files.len(), 3);
    assert_eq!(modified_files.len(), 1);

    // Verify the modified file
    let modified_file = modified_files[0];
    assert_eq!(modified_file.path, "initial.txt");
    assert_eq!(
        modified_file.content.as_ref().expect("Should have content"),
        "Modified in feature branch"
    );

    // Verify added files
    for i in 1..=3 {
        assert!(
            added_files
                .iter()
                .any(|f| f.path == format!("feature{i}.txt"))
        );
    }
}

#[tokio::test]
async fn test_branch_comparison_with_deletions() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = MockDataBuilder::config();

    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create GitTestHelper");

    // Add a file to main branch first using our helper
    helper
        .create_and_stage_file("will_be_deleted.txt", "This will be deleted")
        .expect("Failed to create file to be deleted");

    helper
        .commit("Add file to be deleted")
        .expect("Failed to commit file to delete");

    // Create feature branch from this commit
    helper
        .create_branch("feature-delete")
        .expect("Failed to create feature branch");
    helper
        .checkout_branch("feature-delete")
        .expect("Failed to checkout feature branch");

    // Delete the file using filesystem operations and git
    let file_to_delete_path = temp_dir.path().join("will_be_deleted.txt");
    fs::remove_file(&file_to_delete_path).expect("Failed to remove file");

    let mut index = helper.repo.index().expect("Failed to get repository index");
    index
        .remove_path(Path::new("will_be_deleted.txt"))
        .expect("Failed to remove file from index");
    index.write().expect("Failed to write index");

    // Commit deletion
    helper
        .commit("Delete file")
        .expect("Failed to commit deletion");

    // Test branch comparison
    let context = git_repo
        .get_git_info_for_branch_diff(&config, "main", "feature-delete")
        .expect("Failed to get git info for branch diff");

    // Should have 1 deleted file
    assert_eq!(context.staged_files.len(), 1);
    assert_eq!(context.staged_files[0].path, "will_be_deleted.txt");
    assert!(matches!(
        context.staged_files[0].change_type,
        ChangeType::Deleted
    ));

    // Deleted files shouldn't have content
    assert!(context.staged_files[0].content.is_none());
}

#[tokio::test]
async fn test_branch_comparison_default_from_main() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = MockDataBuilder::config();

    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create GitTestHelper");

    // Create feature branch and switch to it
    helper
        .create_branch("feature-default")
        .expect("Failed to create feature branch");
    helper
        .checkout_branch("feature-default")
        .expect("Failed to checkout feature branch");

    // Add a file using our helper
    helper
        .create_and_stage_file("default_test.txt", "Default from main test")
        .expect("Failed to create and stage feature file");

    helper
        .commit("Add file for default test")
        .expect("Failed to commit feature");

    // Test branch comparison using main as default base
    let context = git_repo
        .get_git_info_for_branch_diff(&config, "main", "feature-default")
        .expect("Failed to get git info for branch diff");

    // Verify we get the expected results
    assert_eq!(context.branch, "main -> feature-default");
    assert_eq!(context.staged_files.len(), 1);
    assert_eq!(context.staged_files[0].path, "default_test.txt");
    assert!(matches!(
        context.staged_files[0].change_type,
        ChangeType::Added
    ));
}

#[tokio::test]
async fn test_branch_comparison_with_binary_files() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = MockDataBuilder::config();

    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create GitTestHelper");

    // Create feature branch and switch to it
    helper
        .create_branch("feature-binary")
        .expect("Failed to create feature branch");
    helper
        .checkout_branch("feature-binary")
        .expect("Failed to checkout feature branch");

    // Add a binary file using our mock data
    let binary_file_path = temp_dir.path().join("binary.png");
    let binary_content = MockDataBuilder::mock_binary_content();
    fs::write(&binary_file_path, binary_content).expect("Failed to write binary file");

    // Stage the binary file (need to use git2 directly for existing files)
    let mut index = helper.repo.index().expect("Failed to get repository index");
    index
        .add_path(Path::new("binary.png"))
        .expect("Failed to add binary file to index");
    index.write().expect("Failed to write index");

    helper
        .commit("Add binary file")
        .expect("Failed to commit binary file");

    // Test branch comparison
    let context = git_repo
        .get_git_info_for_branch_diff(&config, "main", "feature-binary")
        .expect("Failed to get git info for branch diff");

    // Should have 1 binary file
    assert_eq!(context.staged_files.len(), 1);
    assert_eq!(context.staged_files[0].path, "binary.png");
    assert!(matches!(
        context.staged_files[0].change_type,
        ChangeType::Added
    ));

    // Binary files should be detected and marked appropriately
    assert_eq!(context.staged_files[0].diff, "[Binary file changed]");
    assert!(context.staged_files[0].content.is_none());
}

#[tokio::test]
async fn test_branch_comparison_nonexistent_branches() {
    let (_temp_dir, git_repo) = setup_git_repo();
    let config = MockDataBuilder::config();

    // Test with nonexistent base branch
    let result = git_repo.get_git_info_for_branch_diff(&config, "nonexistent-base", "main");
    assert!(result.is_err(), "Should fail with nonexistent base branch");

    // Test with nonexistent target branch
    let result = git_repo.get_git_info_for_branch_diff(&config, "main", "nonexistent-target");
    assert!(
        result.is_err(),
        "Should fail with nonexistent target branch"
    );

    // Test with both branches nonexistent
    let result =
        git_repo.get_git_info_for_branch_diff(&config, "nonexistent-base", "nonexistent-target");
    assert!(
        result.is_err(),
        "Should fail with both branches nonexistent"
    );
}

#[tokio::test]
async fn test_branch_comparison_with_merge_base() {
    let (temp_dir, git_repo) = setup_git_repo();
    let config = MockDataBuilder::config();

    let helper = GitTestHelper::new(&temp_dir).expect("Failed to create GitTestHelper");

    // Create feature branch
    helper
        .create_branch("feature-mergeback")
        .expect("Failed to create feature branch");

    // Add commits to main branch (simulating main moving forward)
    helper
        .create_and_stage_file("main_progress.txt", "Progress in main")
        .expect("Failed to create main progress file");

    let _main_commit = helper
        .commit("Progress in main branch")
        .expect("Failed to commit main progress");

    // Switch to feature branch and add feature work
    helper
        .checkout_branch("feature-mergeback")
        .expect("Failed to checkout feature branch");

    helper
        .create_and_stage_file("feature_work.txt", "Feature work")
        .expect("Failed to create feature work file");

    helper
        .commit("Add feature work")
        .expect("Failed to commit feature work");

    // Test branch comparison (feature branch against main)
    let context = git_repo
        .get_git_info_for_branch_diff(&config, "main", "feature-mergeback")
        .expect("Failed to get git info for branch diff");

    // Should show the feature file added
    assert_eq!(context.staged_files.len(), 1);
    assert_eq!(context.staged_files[0].path, "feature_work.txt");
    assert!(matches!(
        context.staged_files[0].change_type,
        ChangeType::Added
    ));
}
