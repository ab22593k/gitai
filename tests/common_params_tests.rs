use gitai::{common::CommonParams, config::Config};

#[test]
fn test_apply_to_config_model_override() {
    let mut config = Config {
        default_provider: "google".to_string(),
        ..Config::default()
    };

    let common_params = CommonParams {
        provider: None,
        model: Some("gemini-pro".to_string()),
        ..Default::default()
    };

    common_params
        .apply_to_config(&mut config)
        .expect("Failed to apply config");

    let provider_config = config
        .get_provider_config("google")
        .expect("Google config should exist");
    assert_eq!(provider_config.model_name, "gemini-pro");
}

#[test]
fn test_apply_to_config_provider_and_model_override() {
    let mut config = Config::default();

    let common_params = CommonParams {
        provider: Some("google".to_string()),
        model: Some("gemini-1.5-pro".to_string()),
        ..Default::default()
    };

    common_params
        .apply_to_config(&mut config)
        .expect("Failed to apply config");

    assert_eq!(config.default_provider, "google");

    let provider_config = config
        .get_provider_config("google")
        .expect("Google config should exist");
    assert_eq!(provider_config.model_name, "gemini-1.5-pro");
}

#[test]
fn test_apply_to_config_no_override() {
    let mut config = Config::default();
    // Default google model
    let default_model = config
        .get_provider_config("google")
        .expect("Google config should exist")
        .model_name
        .clone();

    let common_params = CommonParams::default();

    common_params
        .apply_to_config(&mut config)
        .expect("Failed to apply config");

    let provider_config = config
        .get_provider_config("google")
        .expect("Google config should exist");
    assert_eq!(provider_config.model_name, default_model);
}
