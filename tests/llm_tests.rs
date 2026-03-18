// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use gitai::core::llm::{
    get_available_provider_names, get_default_model_for_provider, validate_provider_config,
};
use test_utils::MockDataBuilder;

#[test]
fn test_get_available_providers() {
    let providers = get_available_provider_names();
    assert!(providers.len() >= 5);
    assert!(providers.contains(&"google".to_string()));
    assert!(providers.contains(&"groq".to_string()));
    assert!(providers.contains(&"openai".to_string()));
}

#[test]
fn test_get_default_model_for_provider() {
    // Test google provider
    assert_eq!(get_default_model_for_provider("google"), "gemini-2.0-flash");

    // Test groq provider
    assert_eq!(
        get_default_model_for_provider("groq"),
        "llama-3.3-70b-versatile"
    );

    // Test fallback for unknown provider
    assert_eq!(
        get_default_model_for_provider("unknown"),
        "gemini-2.0-flash"
    );
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
