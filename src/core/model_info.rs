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
    ///
    /// Priority:
    /// 1. Valid cached value
    /// 2. API fetch (provider-specific)
    /// 3. Hardcoded fallback
    pub async fn get_token_limit(&self, provider: &str, model: &str, api_key: &str) -> usize {
        let cache_key = format!("{provider}:{model}");

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

        // Try to fetch from API
        let result = self.fetch_model_info(provider, model, api_key).await;

        match result {
            Ok(info) => {
                let context_length = info.context_length;
                debug!("Fetched model info for {cache_key}: {context_length} tokens");

                // Cache the result
                let mut cache = self.cache.write().await;
                cache.insert(cache_key, info);

                context_length
            }
            Err(e) => {
                warn!("Failed to fetch model info for {provider}/{model}: {e}. Using fallback.");
                Self::get_fallback_limit(provider, model)
            }
        }
    }

    /// Fetch model information from the appropriate provider API
    async fn fetch_model_info(
        &self,
        provider: &str,
        model: &str,
        api_key: &str,
    ) -> Result<ModelInfo> {
        match provider.to_lowercase().as_str() {
            "google" => self.fetch_from_google(model, api_key).await,
            "groq" => self.fetch_from_groq(model, api_key).await,
            "openrouter" => self.fetch_from_openrouter(model, api_key).await,
            // Providers without model info APIs use fallback
            "openai" | "anthropic" | "xai" | "deepseek" | "cerebras" | "phind" => Err(
                anyhow::anyhow!("Provider {provider} does not expose model info API"),
            ),
            _ => Err(anyhow::anyhow!("Unknown provider: {provider}")),
        }
    }

    /// Fetch model info from Google Gemini API
    ///
    /// Endpoint: `GET https://generativelanguage.googleapis.com/v1beta/models/{model}`
    async fn fetch_from_google(&self, model: &str, api_key: &str) -> Result<ModelInfo> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}?key={api_key}"
        );

        let response: GoogleModelResponse = self
            .http_client
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

    /// Fetch model info from Groq API
    ///
    /// Endpoint: `GET https://api.groq.com/openai/v1/models`
    async fn fetch_from_groq(&self, model: &str, api_key: &str) -> Result<ModelInfo> {
        let url = "https://api.groq.com/openai/v1/models";

        let response: GroqModelsResponse = self
            .http_client
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

        // Find the model in the response
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

    /// Fetch model info from `OpenRouter` API
    ///
    /// Endpoint: `GET https://openrouter.ai/api/v1/models`
    async fn fetch_from_openrouter(&self, model: &str, api_key: &str) -> Result<ModelInfo> {
        let url = "https://openrouter.ai/api/v1/models";

        let response: OpenRouterModelsResponse = self
            .http_client
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

        // Find the model in the response
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

    /// Get fallback token limit for providers without model info APIs
    /// or when API calls fail
    fn get_fallback_limit(provider: &str, model: &str) -> usize {
        // First try model-specific fallbacks
        if let Some(limit) = Self::get_model_specific_fallback(model) {
            return limit;
        }

        // Then use provider defaults
        match provider.to_lowercase().as_str() {
            "anthropic" => 200_000, // Claude 3 default
            "google" => 1_000_000,  // Gemini default
            "deepseek" => 64_000,
            "phind" => 32_000,
            // OpenAI, OpenRouter, xAI, Cerebras all use 128K context
            "openai" | "openrouter" | "xai" | "cerebras" => 128_000,
            // Default fallback (Groq, unknown providers)
            _ => 8_192,
        }
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

    #[test]
    fn test_fallback_limits() {
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
}
