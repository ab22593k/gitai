use crate::config::Config;
use crate::debug;
use anyhow::{Result, anyhow};
use llm::{
    LLMProvider,
    builder::{LLMBackend, LLMBuilder},
    chat::ChatMessage,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use tokio_retry::Retry;
use tokio_retry::strategy::ExponentialBackoff;

#[derive(Clone, Debug)]
struct ProviderDefault {
    model: &'static str,
    token_limit: usize,
}

static PROVIDER_DEFAULTS: std::sync::LazyLock<HashMap<&'static str, ProviderDefault>> =
    std::sync::LazyLock::new(|| {
        let mut map = HashMap::new();
        map.insert(
            "openai",
            ProviderDefault {
                model: "gpt-4.1",
                token_limit: 128_000,
            },
        );
        map.insert(
            "anthropic",
            ProviderDefault {
                model: "claude-sonnet-4-20250514",
                token_limit: 200_000,
            },
        );
        map.insert(
            "ollama",
            ProviderDefault {
                model: "llama3",
                token_limit: 128_000,
            },
        );
        map.insert(
            "google",
            ProviderDefault {
                model: "gemini-2.5-pro-preview-06-05",
                token_limit: 1_000_000,
            },
        );
        map.insert(
            "groq",
            ProviderDefault {
                model: "llama-3.1-70b-versatile",
                token_limit: 128_000,
            },
        );
        map.insert(
            "xai",
            ProviderDefault {
                model: "grok-2-beta",
                token_limit: 128_000,
            },
        );
        map.insert(
            "deepseek",
            ProviderDefault {
                model: "deepseek-chat",
                token_limit: 64_000,
            },
        );
        map.insert(
            "phind",
            ProviderDefault {
                model: "phind-v2",
                token_limit: 32_000,
            },
        );
        map.insert(
            "openrouter",
            ProviderDefault {
                model: "openrouter/sonoma-dusk-alpha",
                token_limit: 2_000_000,
            },
        );
        map
    });

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
    debug!("Generating message using provider: {}", provider_name);
    debug!("System prompt: {}", system_prompt);
    debug!("User prompt: {}", user_prompt);

    // Parse the provider type
    let backend = if provider_name.to_lowercase() == "openrouter" {
        LLMBackend::OpenRouter
    } else {
        LLMBackend::from_str(provider_name).map_err(|e| anyhow!("Invalid provider: {e}"))?
    };

    // Get provider configuration
    let provider_config = config
        .get_provider_config(provider_name)
        .ok_or_else(|| anyhow!("Provider '{provider_name}' not found in configuration"))?;

    // Build the provider
    let mut builder = LLMBuilder::new().backend(backend.clone());

    // Set model
    if !provider_config.model.is_empty() {
        builder = builder.model(provider_config.model.clone());
    }

    // Set system prompt
    builder = builder.system(system_prompt.to_string());

    // Set API key if needed
    if requires_api_key(&backend) && !provider_config.api_key.is_empty() {
        builder = builder.api_key(provider_config.api_key.clone());
    }

    // Set temperature if specified in additional params
    if let Some(temp) = provider_config.additional_params.get("temperature")
        && let Ok(temp_val) = temp.parse::<f32>()
    {
        builder = builder.temperature(temp_val);
    }

    // Set max tokens if specified in additional params, otherwise use 4096 as default
    // For OpenAI thinking models, don't set max_tokens via builder since they use max_completion_tokens
    if is_openai_thinking_model(&provider_config.model) && provider_name.to_lowercase() == "openai"
    {
        // For thinking models, max_completion_tokens should be handled via additional_params
        // Don't set max_tokens via the builder for these models
    } else if let Some(max_tokens) = provider_config.additional_params.get("max_tokens") {
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
    get_message_with_provider(provider, user_prompt, provider_name).await
}

/// Generates a message using the given provider (mainly for testing purposes)
pub async fn get_message_with_provider<T>(
    provider: Box<dyn LLMProvider + Send + Sync>,
    user_prompt: &str,
    provider_type: &str,
) -> Result<T>
where
    T: DeserializeOwned + JsonSchema,
{
    debug!("Entering get_message_with_provider");

    let retry_strategy = ExponentialBackoff::from_millis(10).factor(2).take(2); // 2 attempts total: initial + 1 retry

    let result = Retry::spawn(retry_strategy, || async {
        debug!("Attempting to generate message");

        // Enhanced prompt that requests specifically formatted JSON output
        let enhanced_prompt = if std::any::type_name::<T>() == std::any::type_name::<String>() {
            user_prompt.to_string()
        } else {
            format!("{user_prompt}\n\nPlease respond with a valid JSON object and nothing else. No explanations or text outside the JSON.")
        };

        // Create chat message with user prompt
        let mut messages = vec![ChatMessage::user().content(enhanced_prompt).build()];

        // Special handling for Anthropic - use the "prefill" technique with "{"
        if provider_type.to_lowercase() == "anthropic" && std::any::type_name::<T>() != std::any::type_name::<String>() {
            messages.push(ChatMessage::assistant().content("Here is the JSON:\n{").build());
        }

        match tokio::time::timeout(Duration::from_secs(30), provider.chat(&messages)).await {
            Ok(Ok(response)) => {
                debug!("Received response from provider");
                let response_text = response.text().unwrap_or_default();

                // Provider-specific response parsing
                let result = match provider_type.to_lowercase().as_str() {
                    // For Anthropic with brace prefixing
                    "anthropic" => {
                        if std::any::type_name::<T>() == std::any::type_name::<String>() {
                            // For String type, we need to handle differently
                            #[allow(clippy::unnecessary_to_owned)]
                            let string_result: T = serde_json::from_value(serde_json::Value::String(response_text.clone()))
                                .map_err(|e| anyhow!("String conversion error: {e}"))?;
                            Ok(string_result)
                        } else {
                            parse_json_response_with_brace_prefix::<T>(&response_text)
                        }
                    },

                    // For all other providers - use appropriate parsing
                    _ => {
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
                };

                match result {
                    Ok(message) => Ok(message),
                    Err(e) => {
                        debug!("JSON parse error: {} text: {}", e, response_text);
                        Err(anyhow!("JSON parse error: {e}"))
                    }
                }
            }
            Ok(Err(e)) => {
                debug!("Provider error: {}", e);
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
            debug!("Failed to generate message after retries: {}", e);
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
            debug!(
                "Direct JSON parse failed: {}. Attempting fallback extraction.",
                e
            );
            extract_and_parse_json(text)
        }
    }
}

/// Parse a response from Anthropic that needs the prefixed "{"
fn parse_json_response_with_brace_prefix<T: DeserializeOwned>(text: &str) -> Result<T> {
    // Add the opening brace that we prefilled in the prompt
    let json_text = format!("{{{text}");
    match serde_json::from_str::<T>(&json_text) {
        Ok(message) => Ok(message),
        Err(e) => {
            debug!(
                "Brace-prefixed JSON parse failed: {}. Attempting fallback extraction.",
                e
            );
            extract_and_parse_json(text)
        }
    }
}

/// Extracts and parses JSON from a potentially non-JSON response
fn extract_and_parse_json<T: DeserializeOwned>(text: &str) -> Result<T> {
    let cleaned_json = clean_json_from_llm(text);
    serde_json::from_str(&cleaned_json).map_err(|e| anyhow!("JSON parse error: {e}"))
}

/// Returns a list of available LLM providers as strings
pub fn get_available_provider_names() -> Vec<String> {
    vec![
        "openai".to_string(),
        "anthropic".to_string(),
        "ollama".to_string(),
        "google".to_string(),
        "groq".to_string(),
        "xai".to_string(),
        "deepseek".to_string(),
        "phind".to_string(),
        "openrouter".to_string(),
    ]
}

/// Returns the default model for a given provider
pub fn get_default_model_for_provider(provider_type: &str) -> &'static str {
    PROVIDER_DEFAULTS
        .get(provider_type.to_lowercase().as_str())
        .map_or("gpt-4.1", |def| def.model)
}

/// Returns the default token limit for a given provider
pub fn get_default_token_limit_for_provider(provider_type: &str) -> usize {
    PROVIDER_DEFAULTS
        .get(provider_type.to_lowercase().as_str())
        .map_or(8_192, |def| def.token_limit)
}

/// Checks if a provider requires an API key
pub fn provider_requires_api_key(provider_type: &str) -> bool {
    if let Ok(backend) = LLMBackend::from_str(provider_type) {
        requires_api_key(&backend)
    } else {
        true // Default to requiring API key for unknown providers
    }
}

/// Helper function: check if `LLMBackend` requires API key
fn requires_api_key(backend: &LLMBackend) -> bool {
    !matches!(backend, LLMBackend::Ollama | LLMBackend::Phind)
}

/// Helper function: check if the model is an `OpenAI` thinking model
fn is_openai_thinking_model(model: &str) -> bool {
    let model_lower = model.to_lowercase();
    model_lower.starts_with('o')
}

/// Validates the provider configuration
pub fn validate_provider_config(config: &Config, provider_name: &str) -> Result<()> {
    if provider_requires_api_key(provider_name) {
        let provider_config = config
            .get_provider_config(provider_name)
            .ok_or_else(|| anyhow!("Provider '{provider_name}' not found in configuration"))?;

        if provider_config.api_key.is_empty() {
            return Err(anyhow!("API key required for provider: {provider_name}"));
        }
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
        if !provider_config.model.is_empty() {
            combined_params.insert("model".to_string(), provider_config.model.clone());
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

    // Handle OpenAI thinking models: convert max_tokens to max_completion_tokens
    if provider_name.to_lowercase() == "openai"
        && let Some(model) = combined_params.get("model")
        && is_openai_thinking_model(model)
        && let Some(max_tokens) = combined_params.remove("max_tokens")
    {
        combined_params.insert("max_completion_tokens".to_string(), max_tokens);
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
