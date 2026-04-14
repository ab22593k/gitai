// Use our centralized test infrastructure
#[path = "../utils_tests.rs"]
mod test_utils;
use gitai::llm::engine::{
    get_available_provider_names, get_default_model_for_provider, validate_provider_config,
};
use test_utils::MockDataBuilder;

#[test]
fn test_get_available_providers() {
    let providers = get_available_provider_names();
    assert_eq!(providers.len(), 2);
    assert!(providers.contains(&"google".to_string()));
    assert!(providers.contains(&"openrouter".to_string()));
}

#[test]
fn test_get_default_model_for_provider() {
    assert_eq!(get_default_model_for_provider("google"), "gemini-2.0-flash");
    assert_eq!(
        get_default_model_for_provider("openrouter"),
        "google/gemini-2.0-flash-001"
    );
    assert_eq!(
        get_default_model_for_provider("unknown"),
        "gemini-2.0-flash"
    );
}

#[test]
fn test_validate_provider_config() {
    let config = MockDataBuilder::test_config_with_api_key("google", "dummy-api-key");
    assert!(validate_provider_config(&config, "google").is_ok());

    let mut invalid_config = config.clone();
    invalid_config
        .providers
        .get_mut("google")
        .expect("Google provider should exist in config")
        .api_key = String::new();
    assert!(validate_provider_config(&invalid_config, "google").is_err());
}
