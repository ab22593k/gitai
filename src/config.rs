use crate::core::llm::{
    get_available_provider_names, get_default_model_for_provider, provider_requires_api_key,
};
use crate::git::GitRepo;

use anyhow::{Context, Result, anyhow};
use git2::Config as GitConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use log::debug;

/// Get a configuration value with layered priority: env var > local git config > global git config
fn get_layered_value(
    key: &str,
    env_var: Option<&str>,
    local_config: Option<&GitConfig>,
    global_config: Option<&GitConfig>,
) -> Option<String> {
    // First, check environment variable
    if let Some(env) = env_var {
        if let Ok(val) = std::env::var(env) {
            return Some(val);
        }
    }

    // Then, check local git config
    if let Some(local) = local_config {
        if let Ok(val) = local.get_string(key) {
            return Some(val.to_string());
        }
    }

    // Finally, check global git config
    if let Some(global) = global_config {
        if let Ok(val) = global.get_string(key) {
            return Some(val.to_string());
        }
    }

    None
}

/// Configuration structure
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    /// Default LLM provider
    pub default_provider: String,
    /// Provider-specific configurations
    pub providers: HashMap<String, ProviderConfig>,
    /// Instructions for commit messages
    #[serde(default)]
    pub instructions: String,
    #[serde(skip)]
    pub temp_instructions: Option<String>,
    /// Flag indicating if this config is local
    #[serde(skip)]
    pub is_local: bool,
}

/// Provider-specific configuration structure
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct ProviderConfig {
    /// API key for the provider
    pub api_key: String,
    /// Model to be used with the provider
    pub model_name: String,
    /// Additional parameters for the provider
    #[serde(default)]
    pub additional_params: HashMap<String, String>,
    /// Token limit, if set by the user
    pub token_limit: Option<usize>,
}

impl Config {
    /// Load the configuration with layered priority: env > local git > global git
    pub fn load() -> Result<Self> {
        // Open git configs
        let global_config = GitConfig::open_default().ok();
        let local_config = git2::Repository::discover(".")
            .ok()
            .and_then(|repo| repo.config().ok());

        let default_provider = get_layered_value(
            "gitai.defaultprovider",
            Some("GITAI_DEFAULT_PROVIDER"),
            local_config.as_ref(),
            global_config.as_ref(),
        ).unwrap_or_else(|| "openai".to_string()); // fallback to openai if not set

        let instructions = get_layered_value(
            "gitai.instructions",
            Some("GITAI_INSTRUCTIONS"),
            local_config.as_ref(),
            global_config.as_ref(),
        ).unwrap_or_default();

        let mut providers = HashMap::new();
        for provider in get_available_provider_names() {
            let api_key_env = match provider.as_str() {
                "openai" => Some("OPENAI_API_KEY"),
                "anthropic" => Some("ANTHROPIC_API_KEY"),
                "google" => Some("GOOGLE_API_KEY"),
                _ => None,
            };

            if let Some(api_key) = get_layered_value(
                &format!("gitai.{provider}-apikey"),
                api_key_env,
                local_config.as_ref(),
                global_config.as_ref(),
            ) {
                let default_model = get_default_model_for_provider(&provider).to_string();
                let model = get_layered_value(
                    &format!("gitai.{provider}-model"),
                    None, // no env for model yet
                    local_config.as_ref(),
                    global_config.as_ref(),
                ).unwrap_or(default_model);

                let token_limit = get_layered_value(
                    &format!("gitai.{provider}-tokenlimit"),
                    None,
                    local_config.as_ref(),
                    global_config.as_ref(),
                ).and_then(|s| s.parse::<i64>().ok())
                .and_then(|v| usize::try_from(v).ok());

                let additional_params = HashMap::new(); // TODO: handle additional params if needed

                providers.insert(
                    provider.to_string(),
                    ProviderConfig {
                        api_key,
                        model_name: model,
                        additional_params,
                        token_limit,
                    },
                );
            }
        }

        let config = Self {
            default_provider,
            providers,
            instructions,
            temp_instructions: None,
            is_local: false,
        };

        debug!("Configuration loaded: {config:?}");
        Ok(config)
    }



    /// Merge this config with project-specific config, with project config taking precedence
    /// But never allow API keys from project config
    pub fn merge_with_project_config(&mut self, project_config: Self) {
        debug!("Merging with project configuration");

        // Override default provider if set in project config
        if project_config.default_provider != Self::default().default_provider {
            self.default_provider
                .clone_from(&project_config.default_provider);
        }

        // Merge provider configs, but never allow API keys from project config
        for (provider, proj_provider_config) in project_config.providers {
            let entry = self.providers.entry(provider).or_default();

            // Don't override API keys from project config (security)
            if !proj_provider_config.model_name.is_empty() {
                entry
                    .model_name
                    .clone_from(&proj_provider_config.model_name);
            }

            // Merge additional params
            entry
                .additional_params
                .extend(proj_provider_config.additional_params);

            // Override token limit if set in project config
            if proj_provider_config.token_limit.is_some() {
                entry.token_limit = proj_provider_config.token_limit;
            }
        }

        // Always override instructions field if set in project config
        self.instructions.clone_from(&project_config.instructions);
    }

    /// Save the configuration to git config
    pub fn save(&self) -> Result<()> {
        // Don't save project configs to personal config file
        if self.is_local {
            return Ok(());
        }

        let mut config = GitConfig::open_default()?;
        self.save_to_config(&mut config, "gitai")?;
        debug!("Configuration saved to global git config: {self:?}");
        Ok(())
    }

    /// Save the configuration to a git config
    fn save_to_config(&self, config: &mut GitConfig, prefix: &str) -> Result<()> {
        // Set default provider
        config.set_str(&format!("{prefix}.defaultprovider"), &self.default_provider)?;

        // Set instructions
        config.set_str(&format!("{prefix}.instructions"), &self.instructions)?;

        for (provider, provider_config) in &self.providers {
            // Set api key only if not empty
            if !provider_config.api_key.is_empty() {
                config.set_str(
                    &format!("{prefix}.{provider}-apikey"),
                    &provider_config.api_key,
                )?;
            }

            // Set model
            config.set_str(
                &format!("{prefix}.{provider}-model"),
                &provider_config.model_name,
            )?;

            if let Some(token_limit) = provider_config.token_limit {
                config.set_i64(
                    &format!("{prefix}.{provider}-tokenlimit"),
                    i64::try_from(token_limit).context("Token limit exceeds i64 range")?,
                )?;
            }

            for (key, value) in &provider_config.additional_params {
                config.set_str(&format!("{prefix}.{provider}-additional{key}"), value)?;
            }
        }

        Ok(())
    }

    /// Save the configuration as a project-specific configuration
    pub fn save_as_project_config(&self) -> Result<()> {
        let repo = git2::Repository::discover(".")?;

        // Before saving, create a copy that excludes API keys
        let mut project_config = self.clone();

        // Remove API keys from all providers
        for provider_config in project_config.providers.values_mut() {
            provider_config.api_key.clear();
        }

        // Mark as project config
        project_config.is_local = true;

        // Save to local git config
        let mut config = repo.config()?;
        project_config.save_to_config(&mut config, "gitai")?;
        debug!("Project configuration saved to local git config: {project_config:?}");
        Ok(())
    }

    /// Check the environment for necessary prerequisites
    pub fn check_environment(&self) -> Result<()> {
        // Check if we're in a git repository
        if !GitRepo::is_inside_work_tree()? {
            return Err(anyhow!(
                "Not in a Git repository. Please run this command from within a Git repository."
            ));
        }

        Ok(())
    }

    #[inline]
    pub fn set_temp_instructions(&mut self, instructions: Option<String>) {
        self.temp_instructions = instructions;
    }

    #[must_use]
    pub fn get_effective_instructions(&self) -> String {
        self.temp_instructions
            .as_ref()
            .unwrap_or(&self.instructions)
            .trim()
            .to_string()
    }

    /// Update the configuration with new values
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        provider: Option<String>,
        api_key: Option<String>,
        model: Option<String>,
        additional_params: Option<HashMap<String, String>>,
        instructions: Option<String>,
        token_limit: Option<usize>,
    ) -> Result<()> {
        if let Some(provider) = provider {
            self.default_provider.clone_from(&provider);
            if !self.providers.contains_key(&provider) {
                // Only insert a new provider if it requires configuration
                if provider_requires_api_key(&provider.to_lowercase()) {
                    self.providers.insert(
                        provider.clone(),
                        ProviderConfig::default_for(&provider.to_lowercase()),
                    );
                }
            }
        }

        let provider_config = self
            .providers
            .get_mut(&self.default_provider)
            .context("Could not get default provider")?;

        if let Some(key) = api_key {
            provider_config.api_key = key;
        }
        if let Some(model) = model {
            provider_config.model_name = model;
        }
        if let Some(params) = additional_params {
            provider_config.additional_params.extend(params);
        }

        if let Some(instr) = instructions {
            self.instructions = instr;
        }
        if let Some(limit) = token_limit {
            provider_config.token_limit = Some(limit);
        }

        debug!("Configuration updated: {self:?}");
        Ok(())
    }

    /// Get the configuration for a specific provider
    #[must_use]
    pub fn get_provider_config(&self, provider: &str) -> Option<&ProviderConfig> {
        // Special case: redirect "claude" to "anthropic"
        let provider_to_lookup = if provider.eq_ignore_ascii_case("claude") {
            "anthropic"
        } else {
            provider
        };

        // First try direct lookup
        self.providers.get(provider_to_lookup).or_else(|| {
            // If not found, try lowercased version
            let lowercase_provider = provider_to_lookup.to_lowercase();

            self.providers.get(&lowercase_provider).or_else(|| {
                // If the provider is not in the config, check if it's a valid provider
                if get_available_provider_names().contains(&lowercase_provider) {
                    // Return None for valid providers not in the config
                    // This allows the code to use default values for providers like Ollama
                    None
                } else {
                    // Return None for invalid providers
                    None
                }
            })
        })
    }

    /// Set whether this config is a project config
    #[inline]
    pub fn set_project_config(&mut self, is_project: bool) {
        self.is_local = is_project;
    }

    /// Check if this is a project config
    #[inline]
    #[must_use]
    pub const fn is_project_config(&self) -> bool {
        self.is_local
    }
}

impl Default for Config {
    fn default() -> Self {
        let providers: HashMap<String, ProviderConfig> = get_available_provider_names()
            .into_iter()
            .map(|provider| (provider.clone(), ProviderConfig::default_for(&provider)))
            .collect();

        // Default to OpenAI if available, otherwise use the first available provider
        let default_provider = if providers.contains_key("openai") {
            "openai".to_string()
        } else {
            providers
                .keys()
                .next()
                .map_or_else(|| "openai".to_string(), std::string::ToString::to_string)
        };

        Self {
            default_provider,
            providers,
            instructions: String::new(),
            temp_instructions: None,
            is_local: false,
        }
    }
}

impl ProviderConfig {
    /// Create a default provider configuration for a given provider
    #[must_use]
    pub fn default_for(provider: &str) -> Self {
        Self {
            api_key: String::new(),
            model_name: get_default_model_for_provider(provider).to_string(),
            additional_params: HashMap::new(),
            token_limit: None, // Will use the default from get_default_token_limit_for_provider
        }
    }

    /// Get the token limit for this provider configuration
    #[inline]
    #[must_use]
    pub const fn get_token_limit(&self) -> Option<usize> {
        self.token_limit
    }
}
