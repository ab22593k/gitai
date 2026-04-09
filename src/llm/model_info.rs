//! Service for fetching model information from LLM provider APIs
//!
//! This module provides dynamic token limit resolution by querying provider APIs
//! to get the actual context window size for a given model, with caching and fallbacks.

use anyhow::{Context, Result};
use log::{debug, warn};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// How long to cache model information (1 hour)
const CACHE_TTL_SECS: u64 = 3600;

/// HTTP request timeout
const REQUEST_TIMEOUT_SECS: u64 = 5;

/// Cached model information
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub model_id: String,
    pub context_length: usize,
    pub max_output_tokens: Option<usize>,
    pub cached_at: Instant,
}

impl ModelInfo {
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > Duration::from_secs(CACHE_TTL_SECS)
    }
}

/// Enum-based dispatch for LLM model providers (avoids dyn compatibility issues with async traits)
#[derive(Debug, Clone, Copy)]
enum ProviderKind {
    Google,
    Groq,
    OpenRouter,
    Anthropic,
    DeepSeek,
    Phind,
    OpenAI,
    Xai,
    Cerebras,
}

impl ProviderKind {
    fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "google" => Some(Self::Google),
            "groq" => Some(Self::Groq),
            "openrouter" => Some(Self::OpenRouter),
            "anthropic" => Some(Self::Anthropic),
            "deepseek" => Some(Self::DeepSeek),
            "phind" => Some(Self::Phind),
            "openai" => Some(Self::OpenAI),
            "xai" => Some(Self::Xai),
            "cerebras" => Some(Self::Cerebras),
            _ => None,
        }
    }

    fn get_fallback_limit(self, _model: &str) -> usize {
        match self {
            Self::Google => 1_000_000,
            Self::Groq => 8_192,
            Self::OpenRouter | Self::OpenAI | Self::Xai | Self::Cerebras => 128_000,
            Self::Anthropic => 200_000,
            Self::DeepSeek => 64_000,
            Self::Phind => 32_000,
        }
    }

    async fn fetch_info(self, client: &Client, model: &str, api_key: &str) -> Result<ModelInfo> {
        match self {
            Self::Google => Self::fetch_google(client, model, api_key).await,
            Self::Groq => Self::fetch_groq(client, model, api_key).await,
            Self::OpenRouter => Self::fetch_openrouter(client, model, api_key).await,
            Self::Anthropic
            | Self::DeepSeek
            | Self::Phind
            | Self::OpenAI
            | Self::Xai
            | Self::Cerebras => Err(anyhow::anyhow!("Provider does not expose model info API")),
        }
    }

    async fn fetch_google(client: &Client, model: &str, api_key: &str) -> Result<ModelInfo> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}?key={api_key}"
        );

        let response: GoogleModelResponse = client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to Google API")?
            .error_for_status()
            .context("Google API returned error status")?
            .json()
            .await
            .context("Failed to parse Google API response")?;

        Ok(ModelInfo {
            model_id: model.to_string(),
            context_length: response.input_token_limit,
            max_output_tokens: Some(response.output_token_limit),
            cached_at: Instant::now(),
        })
    }

    async fn fetch_groq(client: &Client, model: &str, api_key: &str) -> Result<ModelInfo> {
        let url = "https://api.groq.com/openai/v1/models";

        let response: GroqModelsResponse = client
            .get(url)
            .header("Authorization", format!("Bearer {api_key}"))
            .send()
            .await
            .context("Failed to send request to Groq API")?
            .error_for_status()
            .context("Groq API returned error status")?
            .json()
            .await
            .context("Failed to parse Groq API response")?;

        let model_info = response
            .data
            .into_iter()
            .find(|m| m.id == model)
            .ok_or_else(|| anyhow::anyhow!("Model {model} not found in Groq API response"))?;

        Ok(ModelInfo {
            model_id: model.to_string(),
            context_length: model_info.context_window,
            max_output_tokens: None,
            cached_at: Instant::now(),
        })
    }

    async fn fetch_openrouter(client: &Client, model: &str, api_key: &str) -> Result<ModelInfo> {
        let url = "https://openrouter.ai/api/v1/models";

        let response: OpenRouterModelsResponse = client
            .get(url)
            .header("Authorization", format!("Bearer {api_key}"))
            .send()
            .await
            .context("Failed to send request to OpenRouter API")?
            .error_for_status()
            .context("OpenRouter API returned error status")?
            .json()
            .await
            .context("Failed to parse OpenRouter API response")?;

        let model_info = response
            .data
            .into_iter()
            .find(|m| m.id == model)
            .ok_or_else(|| anyhow::anyhow!("Model {model} not found in OpenRouter API response"))?;

        Ok(ModelInfo {
            model_id: model.to_string(),
            context_length: model_info.context_length,
            max_output_tokens: model_info
                .top_provider
                .as_ref()
                .and_then(|p| p.max_completion_tokens),
            cached_at: Instant::now(),
        })
    }
}

/// Service for fetching and caching model information from provider APIs
pub struct ModelInfoService {
    cache: RwLock<HashMap<String, ModelInfo>>,
    http_client: Client,
}

/// Global singleton instance
static MODEL_INFO_SERVICE: OnceLock<ModelInfoService> = OnceLock::new();

impl ModelInfoService {
    /// Create a new `ModelInfoService`
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            http_client: Client::builder()
                .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Get the global singleton instance
    pub fn global() -> &'static ModelInfoService {
        MODEL_INFO_SERVICE.get_or_init(ModelInfoService::new)
    }

    /// Get model info, fetching from API if not cached
    pub async fn get_context_length(
        &self,
        provider_name: &str,
        model: &str,
        api_key: &str,
    ) -> usize {
        let provider_key = provider_name.to_lowercase();
        let cache_key = format!("{provider_key}:{model}");

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(info) = cache.get(&cache_key)
                && !info.is_expired()
            {
                debug!("Cache hit for {cache_key}: {} tokens", info.context_length);
                return info.context_length;
            }
        }

        // Try to fetch from provider
        if let Some(provider) = ProviderKind::from_name(&provider_key) {
            match provider.fetch_info(&self.http_client, model, api_key).await {
                Ok(info) => {
                    let context_length = info.context_length;
                    debug!("Fetched model info for {cache_key}: {context_length} tokens");

                    // Cache the result
                    let mut cache = self.cache.write().await;
                    cache.insert(cache_key, info);

                    return context_length;
                }
                Err(e) => {
                    warn!(
                        "Failed to fetch model info for {provider_key}/{model}: {e}. Using fallback."
                    );
                }
            }
        }

        // Fallback
        Self::get_fallback_limit(&provider_key, model)
    }

    /// Get fallback token limit
    fn get_fallback_limit(provider_name: &str, model: &str) -> usize {
        // First try model-specific fallbacks
        if let Some(limit) = Self::get_model_specific_fallback(model) {
            return limit;
        }

        // Use provider-specific fallback if available
        ProviderKind::from_name(provider_name).map_or(8_192, |p| p.get_fallback_limit(model))
    }

    /// Model-specific fallbacks for known models
    fn get_model_specific_fallback(model: &str) -> Option<usize> {
        let model_lower = model.to_lowercase();

        // OpenAI models
        if model_lower.contains("gpt-4o-mini") {
            return Some(128_000);
        }
        if model_lower.contains("gpt-4o") || model_lower.contains("gpt-4.1") {
            return Some(128_000);
        }
        if model_lower.contains("gpt-4-turbo") {
            return Some(128_000);
        }
        if model_lower.contains("gpt-4") {
            return Some(8_192);
        }
        if model_lower.contains("gpt-3.5") {
            return Some(16_385);
        }
        if model_lower.starts_with("o1") {
            return Some(200_000);
        }

        // Anthropic models
        if model_lower.contains("claude") {
            return Some(200_000);
        }

        // Gemini models
        if model_lower.contains("gemini-1.5") {
            return Some(2_000_000);
        }
        if model_lower.contains("gemini") {
            return Some(1_000_000);
        }

        // Llama models on Groq
        if model_lower.contains("llama") && model_lower.contains("8192") {
            return Some(8_192);
        }
        if model_lower.contains("llama-3.3") || model_lower.contains("llama-3.1") {
            return Some(131_072);
        }

        // Mixtral
        if model_lower.contains("mixtral") {
            return Some(32_768);
        }

        None
    }
}

impl Default for ModelInfoService {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// API Response Structures
// ============================================================================

/// Google Gemini API response for model info
#[derive(Debug, Deserialize)]
struct GoogleModelResponse {
    #[serde(rename = "inputTokenLimit")]
    input_token_limit: usize,
    #[serde(rename = "outputTokenLimit")]
    output_token_limit: usize,
}

/// Groq API response for listing models
#[derive(Debug, Deserialize)]
struct GroqModelsResponse {
    data: Vec<GroqModel>,
}

#[derive(Debug, Deserialize)]
struct GroqModel {
    id: String,
    context_window: usize,
}

/// `OpenRouter` API response for listing models
#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModel {
    id: String,
    context_length: usize,
    top_provider: Option<OpenRouterTopProvider>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterTopProvider {
    max_completion_tokens: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fallback_limits() {
        // Provider defaults
        assert_eq!(
            ModelInfoService::get_fallback_limit("openai", "unknown"),
            128_000
        );
        assert_eq!(
            ModelInfoService::get_fallback_limit("anthropic", "unknown"),
            200_000
        );
        assert_eq!(
            ModelInfoService::get_fallback_limit("google", "unknown"),
            1_000_000
        );
        assert_eq!(
            ModelInfoService::get_fallback_limit("groq", "unknown"),
            8_192
        );

        // Model-specific
        assert_eq!(
            ModelInfoService::get_fallback_limit("openai", "gpt-4o-mini"),
            128_000
        );
        assert_eq!(
            ModelInfoService::get_fallback_limit("anthropic", "claude-3-sonnet"),
            200_000
        );
        assert_eq!(
            ModelInfoService::get_fallback_limit("google", "gemini-1.5-pro"),
            2_000_000
        );
    }

    #[test]
    fn test_model_specific_fallbacks() {
        assert_eq!(
            ModelInfoService::get_model_specific_fallback("gpt-4o"),
            Some(128_000)
        );
        assert_eq!(
            ModelInfoService::get_model_specific_fallback("claude-3-opus"),
            Some(200_000)
        );
        assert_eq!(
            ModelInfoService::get_model_specific_fallback("gemini-2.0-flash"),
            Some(1_000_000)
        );
        assert_eq!(
            ModelInfoService::get_model_specific_fallback("llama3-70b-8192"),
            Some(8_192)
        );
        assert_eq!(
            ModelInfoService::get_model_specific_fallback("unknown-model"),
            None
        );
    }

    #[test]
    fn test_cache_key_format() {
        let provider = "openai";
        let model = "gpt-4o";
        let cache_key = format!("{provider}:{model}");
        assert_eq!(cache_key, "openai:gpt-4o");
    }

    #[test]
    fn test_provider_kind_from_name() {
        assert!(matches!(
            ProviderKind::from_name("google"),
            Some(ProviderKind::Google)
        ));
        assert!(matches!(
            ProviderKind::from_name("Google"),
            Some(ProviderKind::Google)
        ));
        assert!(matches!(
            ProviderKind::from_name("groq"),
            Some(ProviderKind::Groq)
        ));
        assert!(matches!(
            ProviderKind::from_name("openrouter"),
            Some(ProviderKind::OpenRouter)
        ));
        assert!(matches!(
            ProviderKind::from_name("anthropic"),
            Some(ProviderKind::Anthropic)
        ));
        assert!(matches!(
            ProviderKind::from_name("openai"),
            Some(ProviderKind::OpenAI)
        ));
        assert!(ProviderKind::from_name("unknown").is_none());
    }

    #[test]
    fn test_provider_kind_fallback_limits() {
        assert_eq!(ProviderKind::Google.get_fallback_limit("any"), 1_000_000);
        assert_eq!(ProviderKind::Groq.get_fallback_limit("any"), 8_192);
        assert_eq!(ProviderKind::OpenRouter.get_fallback_limit("any"), 128_000);
        assert_eq!(ProviderKind::Anthropic.get_fallback_limit("any"), 200_000);
        assert_eq!(ProviderKind::DeepSeek.get_fallback_limit("any"), 64_000);
        assert_eq!(ProviderKind::Phind.get_fallback_limit("any"), 32_000);
        assert_eq!(ProviderKind::OpenAI.get_fallback_limit("any"), 128_000);
        assert_eq!(ProviderKind::Xai.get_fallback_limit("any"), 128_000);
        assert_eq!(ProviderKind::Cerebras.get_fallback_limit("any"), 128_000);
    }
}
