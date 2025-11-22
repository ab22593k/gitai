use gitai::{common::CommonParams, config::ProviderConfig};
use std::env;
use std::path::Path;
use std::process::Command;

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{MockDataBuilder, setup_git_repo};

// Helper to verify git repo status
fn is_git_repo(dir: &Path) -> bool {
    let status = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(dir)
        .output()
        .expect("Failed to execute git command");

    status.status.success()
}

#[test]
fn test_project_config_security() {
    // Set up a git repository using our centralized infrastructure
    let (temp_dir, _git_repo) = setup_git_repo();

    // Save current directory so we can restore it later
    let original_dir = env::current_dir().expect("Failed to get current directory");

    // Change to the test repository directory for the test
    env::set_current_dir(temp_dir.path()).expect("Failed to change to test directory");

    // Verify we're in a git repo
    assert!(
        is_git_repo(Path::new(".")),
        "Current directory is not a git repository"
    );

    // 1. Test API key security in project config
    // Create a config with API keys using our MockDataBuilder
    let mut config = MockDataBuilder::config();

    // Add API keys to multiple providers
    for provider_name in &["openai", "anthropic", "cohere"] {
        let provider_config = ProviderConfig {
            api_key: format!("secret_{provider_name}_api_key"),
            model_name: format!("{provider_name}_model"),
            ..Default::default()
        };

        config
            .providers
            .insert((*provider_name).to_string(), provider_config);
    }

    // Save as project config
    config
        .save_as_project_config()
        .expect("Failed to save project config");

    // Verify no API keys are in git config
    for provider_name in &["openai", "anthropic", "cohere"] {
        let key = format!("gitai.{provider_name}-apikey");
        let output = Command::new("git")
            .args(["config", "--get", &key])
            .current_dir(".")
            .output()
            .expect("Failed to check git config");
        assert!(
            !output.status.success(),
            "API key was found in git config for provider {provider_name}",
        );
    }

    // 2. Test merging project config with personal config
    // Create configs using our MockDataBuilder
    let mut personal_config =
        MockDataBuilder::test_config_with_api_key("openai", "personal_api_key");
    personal_config
        .providers
        .get_mut("openai")
        .expect("OpenAI provider should exist")
        .model_name = "gpt-3.5-turbo".to_string();

    let mut project_config = MockDataBuilder::config();
    let project_provider_config = ProviderConfig {
        api_key: String::new(), // Empty API key
        model_name: "gpt-4".to_string(),
        ..Default::default()
    };
    project_config
        .providers
        .insert("openai".to_string(), project_provider_config);

    // Merge configs
    personal_config.merge_with_project_config(project_config);

    // Verify API key from personal config is preserved
    let provider_config = personal_config
        .providers
        .get("openai")
        .expect("OpenAI provider config not found");
    assert_eq!(
        provider_config.api_key, "personal_api_key",
        "Personal API key was lost during merge"
    );

    // Verify model from project config is used
    assert_eq!(
        provider_config.model_name, "gpt-4",
        "Project model setting was not applied"
    );

    // 3. Test CLI command integration
    // Set up common parameters similar to CLI arguments
    let common = CommonParams {
        provider: Some("openai".to_string()),
        instructions: Some("Test instructions".to_string()),
        detail_level: "standard".to_string(),
        repository_url: None,
        ..Default::default()
    };

    // Create a config using our MockDataBuilder and apply common parameters
    let mut config = MockDataBuilder::config();
    common
        .apply_to_config(&mut config)
        .expect("Failed to apply common params");

    // Set an API key
    let provider_config = config
        .providers
        .get_mut("openai")
        .expect("OpenAI provider config not found");
    provider_config.api_key = "cli_integration_api_key".to_string();

    // Save as project config
    config
        .save_as_project_config()
        .expect("Failed to save project config with CLI params");

    // Verify the API key is not in git config
    let output = Command::new("git")
        .args(["config", "--get", "gitai.openai-apikey"])
        .current_dir(".")
        .output()
        .expect("Failed to check git config");
    assert!(
        !output.status.success(),
        "API key from CLI integration was found in git config"
    );

    // Clean up - restore original directory
    env::set_current_dir(original_dir).expect("Failed to restore original directory");
}
