//! Central provider registry — single source of truth for all LLM provider identities.
//!
//! This enum consolidates hardcoded provider strings scattered across `engine.rs`,
//! `config.rs`, `model_info.rs`, and command modules. Adding a new provider
//! requires changes only here (and the external `llm` crate's `LLMBackend`).

use llm::builder::LLMBackend;
use std::str::FromStr;

/// All supported LLM providers with their metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    Google,
    Groq,
    OpenRouter,
    OpenAI,
    Anthropic,
    DeepSeek,
    Phind,
    Xai,
    Cerebras,
}

impl ProviderKind {
    /// Parse a provider name from a string (case-insensitive).
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "google" => Some(Self::Google),
            "groq" => Some(Self::Groq),
            "openrouter" => Some(Self::OpenRouter),
            "anthropic" | "claude" => Some(Self::Anthropic),
            "openai" => Some(Self::OpenAI),
            "deepseek" => Some(Self::DeepSeek),
            "phind" => Some(Self::Phind),
            "xai" => Some(Self::Xai),
            "cerebras" => Some(Self::Cerebras),
            _ => None,
        }
    }

    /// The canonical lowercase string name of this provider.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Groq => "groq",
            Self::OpenRouter => "openrouter",
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::DeepSeek => "deepseek",
            Self::Phind => "phind",
            Self::Xai => "xai",
            Self::Cerebras => "cerebras",
        }
    }

    /// The backend type from the `llm` crate used to construct providers.
    pub fn backend(self) -> LLMBackend {
        match self {
            Self::Google => LLMBackend::Google,
            Self::Groq => LLMBackend::Groq,
            // Cerebras not yet in llm crate — use OpenRouter as fallback
            Self::OpenRouter | Self::Cerebras => LLMBackend::OpenRouter,
            Self::OpenAI => LLMBackend::OpenAI,
            Self::Anthropic => LLMBackend::Anthropic,
            Self::DeepSeek => LLMBackend::DeepSeek,
            Self::Phind => LLMBackend::Phind,
            Self::Xai => LLMBackend::XAI,
        }
    }

    /// The default model to use for this provider.
    pub fn default_model(self) -> &'static str {
        match self {
            Self::Google => "gemini-2.0-flash",
            Self::Groq => "llama-3.3-70b-versatile",
            Self::OpenRouter => "google/gemini-2.0-flash-001",
            Self::OpenAI => "gpt-4o",
            Self::Anthropic => "claude-3-5-sonnet-latest",
            Self::DeepSeek => "deepseek-chat",
            Self::Phind => "phind-70b",
            Self::Xai => "grok-2-latest",
            Self::Cerebras => "llama-3.3-70b",
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
            Self::Groq => 8_192,
            Self::OpenRouter | Self::OpenAI | Self::Xai | Self::Cerebras => 128_000,
            Self::Anthropic => 200_000,
            Self::DeepSeek => 64_000,
            Self::Phind => 32_000,
        }
    }

    /// All known providers.
    pub fn all() -> &'static [Self] {
        &[
            Self::Google,
            Self::Groq,
            Self::OpenRouter,
            Self::OpenAI,
            Self::Anthropic,
            Self::DeepSeek,
            Self::Phind,
            Self::Xai,
            Self::Cerebras,
        ]
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
