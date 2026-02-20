#![cfg(feature = "integration")]

use anyhow::Result;
use dotenv::dotenv;
use env_logger;
use gitai::{
    changes::{
        ChangelogGenerator, ReleaseNotesGenerator,
        models::{ChangelogResponse, ReleaseNotesResponse},
    },
    common::DetailLevel,
    config::Config,
};
use git2::Repository;

use std::env;
use tempfile::TempDir;

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::setup_git_repo_with_tags;

fn setup_test_repo() -> Result<(TempDir, Repository)> {
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Stdout)
        .init();

    setup_git_repo_with_tags()
}

fn setup_config() -> Config {
    dotenv().ok();
    let mut config = Config {
        default_provider: "openai".to_string(),
        ..Default::default()
    };
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    config
        .providers
        .get_mut(&config.default_provider)
        .expect("OpenAI provider config not found")
        .api_key = api_key;
    config
}

#[tokio::test]
async fn test_changelog_generation() -> Result<()> {
    let (temp_dir, _repo) = setup_test_repo()?;
    let config = setup_config();

    let repo_path = std::sync::Arc::new(gitai::git::GitRepo::new(temp_dir.path())?);
    let changelog = ChangelogGenerator::generate(
        repo_path,
        "v1.0.0",
        "v1.1.0",
        &config,
        DetailLevel::Standard,
    )
    .await?;

    let changelog_response: ChangelogResponse = serde_json::from_str(&changelog)?;

    assert!(
        changelog_response.version.is_some(),
        "Changelog should have a version"
    );
    assert!(
        changelog_response.release_date.is_some(),
        "Changelog should have a release date"
    );
    assert!(
        !changelog_response.sections.is_empty(),
        "Changelog should have sections"
    );
    assert!(
        changelog_response.metrics.total_commits > 0,
        "Changelog should have commits"
    );
    assert!(
        changelog_response.metrics.files_changed > 0,
        "Changelog should have file changes"
    );

    Ok(())
}

#[tokio::test]
async fn test_release_notes_generation() -> Result<()> {
    let (temp_dir, _repo) = setup_test_repo()?;
    let config = setup_config();

    let repo_path = std::sync::Arc::new(gitai::git::GitRepo::new(temp_dir.path())?);
    let release_notes = ReleaseNotesGenerator::generate(
        repo_path,
        "v1.0.0",
        "v1.1.0",
        &config,
        DetailLevel::Standard,
        None,
    )
    .await?;

    let release_notes_response: ReleaseNotesResponse = serde_json::from_str(&release_notes)?;

    assert!(
        release_notes_response.version.is_some(),
        "Release notes should have a version"
    );
    assert!(
        release_notes_response.release_date.is_some(),
        "Release notes should have a release date"
    );
    assert!(
        !release_notes_response.summary.is_empty(),
        "Release notes should have a summary"
    );
    assert!(
        release_notes_response.metrics.total_commits > 0,
        "Release notes should have commits"
    );
    assert!(
        release_notes_response.metrics.files_changed > 0,
        "Release notes should have file changes"
    );

    Ok(())
}

#[tokio::test]
async fn test_changelog_generation_with_custom_version() -> Result<()> {
    let (temp_dir, _repo) = setup_test_repo()?;
    let config = setup_config();
    let custom_version = "v2.0.0-beta";

    // We need to provide a path to GitRepo for this integration test
    let repo_path = std::sync::Arc::new(gitai::git::GitRepo::new(temp_dir.path())?);

    // Generate changelog with custom version name
    let changelog = ChangelogGenerator::generate(
        repo_path.clone(),
        "v1.0.0",
        "v1.1.0",
        &config,
        DetailLevel::Standard,
    )
    .await?;

    // Generate a temporary changelog file using the custom version
    let changelog_path = temp_dir.path().join("CHANGELOG.md");
    ChangelogGenerator::update_changelog_file(
        &changelog,
        changelog_path
            .to_str()
            .expect("Invalid path for changelog file"),
        &repo_path,
        "HEAD",
        Some(custom_version.to_string()),
    )?;

    // Read the content to verify the custom version was used
    let content = std::fs::read_to_string(&changelog_path)?;
    assert!(
        content.contains(&format!("## [{custom_version}]")),
        "Changelog should contain the custom version name"
    );

    Ok(())
}

#[tokio::test]
async fn test_release_notes_generation_with_custom_version() -> Result<()> {
    let (temp_dir, _repo) = setup_test_repo()?;
    let config = setup_config();
    let custom_version = "v2.0.0-rc1";

    // We need to provide a path to GitRepo for this integration test
    let repo_path = std::sync::Arc::new(gitai::git::GitRepo::new(temp_dir.path())?);

    // Generate release notes with custom version name
    let release_notes = ReleaseNotesGenerator::generate(
        repo_path,
        "v1.0.0",
        "v1.1.0",
        &config,
        DetailLevel::Standard,
        Some(custom_version.to_string()),
    )
    .await?;

    // Verify the custom version was used
    assert!(
        release_notes.contains(&format!("Release Notes - v{custom_version}")),
        "Release notes should contain the custom version name"
    );

    Ok(())
}
