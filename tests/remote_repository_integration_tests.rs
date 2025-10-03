#![cfg(feature = "integration")]

use anyhow::Result;
use gitpilot::cli::Commands;
use gitpilot::common::CommonParams;
use gitpilot::git::GitRepo;
use std::env;

// Test the CLI with a remote repository URL
#[tokio::test]
async fn test_cli_with_remote_repository() -> Result<()> {
    // Skip this test in CI environments or when no network is available
    if env::var("CI").is_ok() || env::var("SKIP_REMOTE_TESTS").is_ok() {
        return Ok(());
    }

    // Test a public repository URL that is unlikely to disappear
    let repo_url = "https://github.com/rust-lang/rust.git";

    // First, verify that the URL is valid and can be cloned
    let git_repo = GitRepo::new_from_url(Some(repo_url.to_string()))?;
    assert!(
        git_repo.is_remote(),
        "Repository should be marked as remote"
    );

    // 1. Test ReleaseNotes command with repository URL
    let common = CommonParams {
        provider: Some("mock".to_string()), // Use mock provider to avoid real API calls
        instructions: None,
        preset: None,
        gitmoji: Some(false),
        detail_level: "minimal".to_string(),
        repository_url: Some(repo_url.to_string()),
    };

    let release_notes_command = Commands::ReleaseNotes {
        common: common.clone(),
        from: "v1.0.0".to_string(), // Use a tag that's likely to exist in the repo
        to: Some("HEAD".to_string()),
        version_name: None,
    };

    // Just testing that it doesn't panic, we're not making actual API calls
    let result = gitv::cli::handle_command(release_notes_command, None).await;
    assert!(
        result.is_err(),
        "Command should fail because we're using a mock provider"
    );

    // 2. Test Changelog command with repository URL
    let changelog_command = Commands::Changelog {
        common: common.clone(),
        from: "v1.0.0".to_string(),
        to: Some("HEAD".to_string()),
        file: None,
        update: false,
        version_name: None,
    };

    // Just testing that it doesn't panic
    let result = gitv::cli::handle_command(changelog_command, None).await;
    assert!(
        result.is_err(),
        "Command should fail because we're using a mock provider"
    );

    // 3. Test Review command with repository URL
    let review_command = Commands::Review {
        common: common.clone(),
        print: true,
        commit: None,
        include_unstaged: false,
        from: None,
        to: None,
    };

    // Just testing that it doesn't panic
    let result = gitv::cli::handle_command(review_command, None).await;
    assert!(
        result.is_err(),
        "Command should fail because we're using a mock provider"
    );

    // 4. Test cmsg command with repository URL
    let gen_command = Commands::Msg {
        common,
        auto_commit: false,
        no_gitmoji: true,
        print: true,
        no_verify: true,
    };

    // Just testing that it doesn't panic
    let result = gitv::cli::handle_command(gen_command, None).await;
    assert!(
        result.is_err(),
        "Command should fail because we're using a mock provider"
    );

    Ok(())
}
