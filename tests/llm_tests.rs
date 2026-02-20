// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use gitai::core::llm::{
    get_available_provider_names, get_default_model_for_provider,
    get_default_token_limit_for_provider, validate_provider_config,
};
use test_utils::MockDataBuilder;

#[test]
fn test_get_available_providers() {
    let providers = get_available_provider_names();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0], "google");
}

#[test]
fn test_get_default_model_for_provider() {
    // Test google provider
    assert_eq!(
        get_default_model_for_provider("google"),
        "gemini-2.5-flash-lite"
    );

    // Test fallback for unknown provider
    assert_eq!(
        get_default_model_for_provider("unknown"),
        "gemini-2.5-flash-lite"
    );
}

#[test]
fn test_get_default_token_limit_for_provider() {
    // Test google provider
    assert_eq!(get_default_token_limit_for_provider("google"), 1_000_000);

    // Test fallback for unknown provider
    assert_eq!(get_default_token_limit_for_provider("unknown"), 1_000_000);
}

#[test]
fn test_validate_provider_config() {
    // Create a config with valid provider configuration using our MockDataBuilder
    let config = MockDataBuilder::test_config_with_api_key("google", "dummy-api-key");

    // Validation should pass with API key set
    assert!(validate_provider_config(&config, "google").is_ok());

    // Test with missing API key
    let mut invalid_config = config.clone();
    invalid_config
        .providers
        .get_mut("google")
        .expect("Google provider should exist in config")
        .api_key = String::new();
    assert!(validate_provider_config(&invalid_config, "google").is_err());
}
