use crate::config::Config;
use anyhow::{Result, anyhow};
use llm::{
    LLMProvider,
    builder::{LLMBackend, LLMBuilder},
    chat::ChatMessage,
};
use log::debug;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::time::Duration;
use tokio_retry::Retry;
use tokio_retry::strategy::ExponentialBackoff;
use tracing::Level;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::format::FmtSpan;

#[derive(Debug)]
struct ProviderDefault {
    model: &'static str,
    token_limit: usize,
}

static PROVIDER_DEFAULTS: std::sync::LazyLock<
    std::collections::HashMap<&'static str, ProviderDefault>,
> = std::sync::LazyLock::new(|| {
    let mut m = std::collections::HashMap::new();
    m.insert(
        "google",
        ProviderDefault {
            model: "gemini-2.5-flash-lite",
            token_limit: 1_000_000,
        },
    );
    m
});

/// Initialize tracing to a rolling file in target/debug
pub fn init_tracing_to_file() {
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "target/debug", "llm-debug.log");
    tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_max_level(Level::INFO)
        .with_span_events(FmtSpan::CLOSE)
        .json()
        .init();
}

/// Generates a message using the given configuration
pub async fn get_message<T>(
    config: &Config,
    provider_name: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<T>
where
    T: DeserializeOwned + JsonSchema,
{
    debug!("Generating message using provider: {provider_name}");
    debug!("System prompt: {system_prompt}");
    debug!("User prompt: {user_prompt}");

    // Only Google is supported
    if provider_name.to_lowercase() != "google" {
        return Err(anyhow!("Only Google provider is supported"));
    }

    let backend = LLMBackend::Google;

    // Get provider configuration
    let provider_config = config
        .get_provider_config(provider_name)
        .ok_or_else(|| anyhow!("Provider '{provider_name}' not found in configuration"))?;

    // Build the provider
    let mut builder = LLMBuilder::new().backend(backend.clone());

    // Set model
    if !provider_config.model_name.is_empty() {
        builder = builder.model(provider_config.model_name.clone());
    }

    // Set system prompt
    builder = builder.system(system_prompt.to_string());

    // Set API key if needed
    if !provider_config.api_key.is_empty() {
        builder = builder.api_key(provider_config.api_key.clone());
    }

    // Set temperature if specified in additional params
    if let Some(temp) = provider_config.additional_params.get("temperature")
        && let Ok(temp_val) = temp.parse::<f32>()
    {
        builder = builder.temperature(temp_val);
    }

    // Set max tokens if specified in additional params, otherwise use provider default
    if let Some(max_tokens) = provider_config.additional_params.get("max_tokens") {
        if let Ok(mt_val) = max_tokens.parse::<u32>() {
            builder = builder.max_tokens(mt_val);
        }
    } else {
        let default_max = get_default_token_limit_for_provider(provider_name)
            .try_into()
            .map_err(|e| anyhow!("Token limit too large for u32: {e}"))?;
        builder = builder.max_tokens(default_max);
    }

    // Set top_p if specified in additional params
    if let Some(top_p) = provider_config.additional_params.get("top_p")
        && let Ok(tp_val) = top_p.parse::<f32>()
    {
        builder = builder.top_p(tp_val);
    }

    // Build the provider
    let provider = builder
        .build()
        .map_err(|e| anyhow!("Failed to build provider: {e}"))?;

    // Generate the message
    get_message_with_provider(provider, user_prompt, provider_name, system_prompt).await
}

/// Generates a message using the given provider (mainly for testing purposes)
pub async fn get_message_with_provider<T>(
    provider: Box<dyn LLMProvider + Send + Sync>,
    user_prompt: &str,
    _provider_type: &str,
    #[allow(clippy::used_underscore_binding)] _system_prompt: &str,
) -> Result<T>
where
    T: DeserializeOwned + JsonSchema,
{
    debug!("Entering get_message_with_provider");

    let retry_strategy = ExponentialBackoff::from_millis(50).factor(2).take(3); // 3 attempts total: initial + 2 retries

    let result = Retry::spawn(retry_strategy, || async {
        debug!("Attempting to generate message");

        // Enhanced prompt that requests specifically formatted JSON output
        let enhanced_prompt = if std::any::type_name::<T>() == std::any::type_name::<String>() {
            user_prompt.to_string()
        } else {
            format!("{user_prompt}\n\nPlease respond with a valid JSON object and nothing else. No explanations or text outside the JSON.")
        };

        // Create chat message with user prompt
        let messages = vec![ChatMessage::user().content(enhanced_prompt.clone()).build()];

        match tokio::time::timeout(Duration::from_secs(60), provider.chat(&messages)).await {
            Ok(Ok(response)) => {
                let response_text = response.text().unwrap_or_default();
                debug!("Received response from provider");

                if std::any::type_name::<T>() == std::any::type_name::<String>() {
                    // For String type, we need to handle differently
                    #[allow(clippy::unnecessary_to_owned)]
                    let string_result: T = serde_json::from_value(serde_json::Value::String(response_text.clone()))
                        .map_err(|e| anyhow!("String conversion error: {e}"))?;
                    Ok(string_result)
                } else {
                    // First try direct parsing, then fall back to extraction
                    parse_json_response::<T>(&response_text)
                }
            }
            Ok(Err(e)) => {
                debug!("Provider error: {e}");
                Err(anyhow!("Provider error: {e}"))
            }
            Err(_) => {
                debug!("Provider timed out");
                Err(anyhow!("Provider timed out"))
            }
        }
    })
    .await;

    match result {
        Ok(message) => {
            debug!("Generated message successfully");
            Ok(message)
        }
        Err(e) => {
            debug!("Failed to generate message after retries: {e}");
            Err(anyhow!("Failed to generate message: {e}"))
        }
    }
}

/// Parse a provider's response that should be pure JSON
fn parse_json_response<T: DeserializeOwned>(text: &str) -> Result<T> {
    match serde_json::from_str::<T>(text) {
        Ok(message) => Ok(message),
        Err(e) => {
            // Fallback to a more robust extraction if direct parsing fails
            debug!("Direct JSON parse failed: {e}. Attempting fallback extraction.");
            extract_and_parse_json(text)
        }
    }
}

/// Extracts and parses JSON from a potentially non-JSON response
fn extract_and_parse_json<T: DeserializeOwned>(text: &str) -> Result<T> {
    let cleaned_json = clean_json_from_llm(text);
    serde_json::from_str(&cleaned_json).map_err(|e| anyhow!("JSON parse error: {e}"))
}

pub fn get_available_provider_names() -> Vec<String> {
    vec!["google".to_string()]
}

/// Returns the default model for a given provider
pub fn get_default_model_for_provider(provider_type: &str) -> &'static str {
    PROVIDER_DEFAULTS
        .get(provider_type.to_lowercase().as_str())
        .map_or("gemini-2.5-flash-lite", |def| def.model)
}

/// Returns the default token limit for a given provider
pub fn get_default_token_limit_for_provider(provider_type: &str) -> usize {
    PROVIDER_DEFAULTS
        .get(provider_type.to_lowercase().as_str())
        .map_or(1_000_000, |def| def.token_limit)
}

/// Checks if a provider requires an API key
pub fn provider_requires_api_key(_provider_type: &str) -> bool {
    true
}

/// Validates the provider configuration
pub fn validate_provider_config(config: &Config, provider_name: &str) -> Result<()> {
    let provider_config = config
        .get_provider_config(provider_name)
        .ok_or_else(|| anyhow!("Provider '{provider_name}' not found in configuration"))?;

    if provider_config.api_key.is_empty() {
        return Err(anyhow!("API key required for provider: {provider_name}"));
    }

    Ok(())
}

/// Combines default, saved, and command-line configurations
pub fn get_combined_config<S: ::std::hash::BuildHasher>(
    config: &Config,
    provider_name: &str,
    command_line_args: &HashMap<String, String, S>,
) -> HashMap<String, String> {
    let mut combined_params = HashMap::default();

    // Add default values
    combined_params.insert(
        "model".to_string(),
        get_default_model_for_provider(provider_name).to_string(),
    );

    // Add saved config values if available
    if let Some(provider_config) = config.get_provider_config(provider_name) {
        if !provider_config.api_key.is_empty() {
            combined_params.insert("api_key".to_string(), provider_config.api_key.clone());
        }
        if !provider_config.model_name.is_empty() {
            combined_params.insert("model".to_string(), provider_config.model_name.clone());
        }
        for (key, value) in &provider_config.additional_params {
            combined_params.insert(key.clone(), value.clone());
        }
    }

    // Add command line args (these take precedence)
    for (key, value) in command_line_args {
        if !value.is_empty() {
            combined_params.insert(key.clone(), value.clone());
        }
    }

    combined_params
}

fn clean_json_from_llm(json_str: &str) -> String {
    // Remove potential leading/trailing whitespace and invisible characters
    let trimmed = json_str
        .trim_start_matches(|c: char| c.is_whitespace() || !c.is_ascii())
        .trim_end_matches(|c: char| c.is_whitespace() || !c.is_ascii());

    // If wrapped in code block, remove the markers
    let without_codeblock = if trimmed.starts_with("```") && trimmed.ends_with("```") {
        let start = trimmed.find('{').unwrap_or(0);
        let end = trimmed.rfind('}').map_or(trimmed.len(), |i| i + 1);
        &trimmed[start..end]
    } else {
        trimmed
    };

    // Find the first '{' and last '}' to extract the JSON object
    let start = without_codeblock.find('{').unwrap_or(0);
    let end = without_codeblock
        .rfind('}')
        .map_or(without_codeblock.len(), |i| i + 1);

    without_codeblock[start..end].trim().to_string()
}
