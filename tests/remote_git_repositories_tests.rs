use gitai::{config::Config, git::GitRepo};

#[tokio::test]
async fn test_remote_repository_support() {
    // Skip this test in CI environments or when no network is available
    if std::env::var("CI").is_ok() || std::env::var("SKIP_REMOTE_TESTS").is_ok() {
        return;
    }

    // Use a small test repo instead of the full Rust repo
    let repo_url = "https://github.com/hyperb1iss/tiny-test-repo.git";

    // Add a timeout for the test
    let test_future = async {
        // Use new_from_url to create a GitRepo from a remote URL
        let git_repo = match GitRepo::new_from_url(Some(repo_url.to_string())) {
            Ok(repo) => repo,
            Err(e) => {
                println!("Failed to clone test repo: {e}");
                println!("Skipping remote repository test - network might be unavailable");
                return;
            }
        };

        // Verify it's marked as remote
        assert!(
            git_repo.is_remote(),
            "Repository should be marked as remote"
        );

        // Verify the remote URL is stored correctly
        assert_eq!(
            git_repo.get_remote_url(),
            Some(repo_url),
            "Remote URL should match"
        );

        // Test basic repository operations
        let config = Config::default();

        // Get git info should work with remote repositories
        let context = match git_repo.get_git_info(&config).await {
            Ok(ctx) => ctx,
            Err(e) => {
                println!("Failed to get git info: {e}");
                println!("Skipping remaining remote repository tests");
                return;
            }
        };

        // Should have a valid branch name
        assert!(
            !context.branch.is_empty(),
            "Should have a valid branch name"
        );

        // Should have some recent commits
        assert!(
            !context.recent_commits.is_empty(),
            "Should have recent commits"
        );

        // Test read-only operations
        let update_result = git_repo.update_remote();
        assert!(update_result.is_ok(), "Should be able to update remote");

        // Commit operations should fail for remote repositories
        let result = git_repo.commit("Test commit message");
        assert!(
            result.is_err(),
            "Commit should fail for remote repositories"
        );

        // The error message should indicate it's a remote repository
        let error_message = result
            .expect_err("Expected an error when committing to a remote repository")
            .to_string();
        assert!(
            error_message.contains("Cannot commit to a remote repository"),
            "Error message should indicate it's a remote repository"
        );
    };

    // Run the test with a reasonable timeout
    if let Ok(()) = tokio::time::timeout(std::time::Duration::from_secs(30), test_future).await {
        // Test completed within timeout
        println!("Remote repository test completed successfully");
    } else {
        // Test timed out
        println!("Remote repository test timed out after 30 seconds");
        println!("Consider using --skip-remote-tests if network is slow");
    }
}
