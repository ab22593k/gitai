use gait::{common::CommonParams, config::Config};

#[test]
fn test_apply_to_config_model_override() {
    let mut config = Config::default();
    
    // Ensure default provider is openai for this test
    config.default_provider = "openai".to_string();
    
    let common_params = CommonParams {
        provider: None,
        model: Some("gpt-4o".to_string()),
        ..Default::default()
    };

    common_params.apply_to_config(&mut config).expect("Failed to apply config");

    let provider_config = config.get_provider_config("openai").expect("OpenAI config should exist");
    assert_eq!(provider_config.model_name, "gpt-4o");
}

#[test]
fn test_apply_to_config_provider_and_model_override() {
    let mut config = Config::default();
    
    let common_params = CommonParams {
        provider: Some("anthropic".to_string()),
        model: Some("claude-3-opus-20240229".to_string()),
        ..Default::default()
    };

    common_params.apply_to_config(&mut config).expect("Failed to apply config");

    assert_eq!(config.default_provider, "anthropic");
    
    let provider_config = config.get_provider_config("anthropic").expect("Anthropic config should exist");
    assert_eq!(provider_config.model_name, "claude-3-opus-20240229");
}

#[test]
fn test_apply_to_config_no_override() {
    let mut config = Config::default();
    // Default openai model
    let default_model = config.get_provider_config("openai").unwrap().model_name.clone();

    let common_params = CommonParams::default();

    common_params.apply_to_config(&mut config).expect("Failed to apply config");

    let provider_config = config.get_provider_config("openai").expect("OpenAI config should exist");
    assert_eq!(provider_config.model_name, default_model);
}
