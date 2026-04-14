//! Central provider registry — single source of truth for all LLM provider identities.
//!
//! This enum consolidates provider identities. Adding a new provider
//! requires changes only here (and the external `llm` crate's `LLMBackend`).

use llm::builder::LLMBackend;
use std::str::FromStr;

/// All supported LLM providers with their metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    Google,
    OpenRouter,
}

impl ProviderKind {
    /// Parse a provider name from a string (case-insensitive).
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "google" => Some(Self::Google),
            "openrouter" => Some(Self::OpenRouter),
            _ => None,
        }
    }

    /// The canonical lowercase string name of this provider.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::OpenRouter => "openrouter",
        }
    }

    /// The backend type from the `llm` crate used to construct providers.
    pub fn backend(self) -> LLMBackend {
        match self {
            Self::Google => LLMBackend::Google,
            Self::OpenRouter => LLMBackend::OpenRouter,
        }
    }

    /// The default model to use for this provider.
    pub fn default_model(self) -> &'static str {
        match self {
            Self::Google => "gemini-2.0-flash",
            Self::OpenRouter => "google/gemini-2.0-flash-001",
        }
    }

    /// Whether this provider requires an API key.
    pub const fn requires_api_key(self) -> bool {
        true
    }

    /// Fallback context window for model info when the provider doesn't expose an API.
    pub fn model_info_fallback_limit(self) -> usize {
        match self {
            Self::Google => 1_000_000,
            Self::OpenRouter => 128_000,
        }
    }

    /// All known providers.
    pub fn all() -> &'static [Self] {
        &[Self::Google, Self::OpenRouter]
    }
}

impl FromStr for ProviderKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(s).ok_or_else(|| anyhow::anyhow!("Unknown provider: {s}"))
    }
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
