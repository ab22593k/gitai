use crate::core::llm::{get_available_provider_names, get_default_model_for_provider};
use crate::git::GitRepo;

use anyhow::{Result, anyhow};
use git2::Config as GitConfig;
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Get a configuration value with layered priority: env var > local git config > global git config
fn get_layered_value(
    key: &str,
    env_var: Option<&str>,
    local_config: Option<&GitConfig>,
    global_config: Option<&GitConfig>,
) -> Option<String> {
    // First, check environment variable
    if let Some(env) = env_var
        && let Ok(val) = std::env::var(env)
    {
        return Some(val);
    }

    // Then, check local git config
    if let Some(local) = local_config
        && let Ok(val) = local.get_string(key)
    {
        return Some(val.clone());
    }

    // Finally, check global git config
    if let Some(global) = global_config
        && let Ok(val) = global.get_string(key)
    {
        return Some(val.clone());
    }

    None
}

/// Load additional parameters for a provider from Git config
fn load_additional_params(
    config: &GitConfig,
    provider: &str,
    additional_params: &mut HashMap<String, String>,
) {
    let prefix = format!("gitai.{provider}-additional");
    if let Ok(mut entries) = config.entries(Some(&prefix)) {
        while let Some(Ok(entry)) = entries.next() {
            if let Some(name) = entry.name()
                && let Some(value) = entry.value()
                && name.starts_with(&prefix)
            {
                let key = name[prefix.len()..].to_string();
                if !key.is_empty() {
                    additional_params.insert(key, value.to_string());
                }
            }
        }
    }
}

/// Get the environment variable name for a provider's API key
fn get_api_key_env_var(provider: &str) -> Option<&'static str> {
    match provider {
        "google" => Some("GOOGLE_API_KEY"),
        _ => None,
    }
}

/// Configuration structure
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
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
    #[serde(skip)]
    pub api_key: String,
    /// Model to be used with the provider
    pub model_name: String,
    /// Additional parameters for the provider
    #[serde(default)]
    pub additional_params: HashMap<String, String>,
}

impl Config {
    /// Load the configuration with layered priority: env > local git > global git
    ///
    /// # Errors
    ///
    /// Returns an error if the git configuration cannot be accessed.
    pub fn load() -> Result<Self> {
        // Open git configs
        let global_config = GitConfig::open_default().ok();
        let local_config = git2::Repository::discover(".")
            .ok()
            .and_then(|repo| repo.config().ok());

        let instructions = get_layered_value(
            "gitai.instructions",
            Some("GITAI_INSTRUCTIONS"),
            local_config.as_ref(),
            global_config.as_ref(),
        )
        .unwrap_or_default();

        let mut providers = HashMap::new();
        for provider in get_available_provider_names() {
            let api_key = get_layered_value(
                &format!("gitai.{provider}-apikey"),
                get_api_key_env_var(&provider),
                local_config.as_ref(),
                global_config.as_ref(),
            )
            .unwrap_or_default();

            let default_model = get_default_model_for_provider(&provider).to_string();
            let model = get_layered_value(
                &format!("gitai.{provider}-model"),
                None, // no env for model yet
                local_config.as_ref(),
                global_config.as_ref(),
            )
            .unwrap_or(default_model);

            let mut additional_params = HashMap::new();
            // Load from global first, then local to allow local to override
            if let Some(ref config) = global_config {
                load_additional_params(config, &provider, &mut additional_params);
            }
            if let Some(ref config) = local_config {
                load_additional_params(config, &provider, &mut additional_params);
            }

            providers.insert(
                #[allow(clippy::implicit_clone)]
                provider.to_owned(),
                ProviderConfig {
                    api_key,
                    model_name: model,
                    additional_params,
                },
            );
        }

        let config = Self {
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
        }

        // Always override instructions field if set in project config
        self.instructions.clone_from(&project_config.instructions);
    }

    /// Save the configuration to git config
    ///
    /// # Errors
    ///
    /// Returns an error if the git configuration cannot be written.
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
        // Set instructions
        config.set_str(&format!("{prefix}.instructions"), &self.instructions)?;

        for (provider, provider_config) in &self.providers {
            // Set model
            config.set_str(
                &format!("{prefix}.{provider}-model"),
                &provider_config.model_name,
            )?;

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
        api_key: Option<String>,
        model: Option<String>,
        additional_params: Option<HashMap<String, String>>,
        instructions: Option<String>,
    ) -> Result<()> {
        let provider_name = "google".to_string();

        if let Some(key) = api_key {
            let entry = self.providers.entry(provider_name.clone()).or_default();
            entry.api_key = key;
        }

        if let Some(model) = model {
            let entry = self.providers.entry(provider_name.clone()).or_default();
            entry.model_name = model;
        }

        if let Some(params) = additional_params
            && let Some(provider_config) = self.providers.get_mut(&provider_name)
        {
            provider_config.additional_params.extend(params);
        }

        if let Some(instr) = instructions {
            self.instructions = instr;
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

        // Try direct lookup first, then lowercase
        self.providers
            .get(provider_to_lookup)
            .or_else(|| self.providers.get(&provider_to_lookup.to_lowercase()))
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

        Self {
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
        }
    }
}
