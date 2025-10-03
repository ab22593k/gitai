use crate::git::GitRepo;
use crate::instruction_presets::get_instruction_preset_library;
use crate::llm::{
    get_available_provider_names, get_default_model_for_provider, provider_requires_api_key,
};
use crate::log_debug;

use anyhow::{Context, Result, anyhow};
use git2::Config as GitConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;

/// Configuration structure
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    /// Default LLM provider
    pub default_provider: String,
    /// Provider-specific configurations
    pub providers: HashMap<String, ProviderConfig>,
    /// Flag indicating whether to use emoji
    #[serde(default = "default_emoji")]
    pub use_emoji: bool,
    /// Instructions for commit messages
    #[serde(default)]
    pub instructions: String,
    #[serde(default = "default_instruction_preset")]
    pub instruction_preset: String,
    #[serde(skip)]
    pub temp_instructions: Option<String>,
    #[serde(skip)]
    pub temp_preset: Option<String>,
    /// Flag indicating if this config is from a project file
    #[serde(skip)]
    pub is_project_config: bool,
}

/// Provider-specific configuration structure
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct ProviderConfig {
    /// API key for the provider
    pub api_key: String,
    /// Model to be used with the provider
    pub model: String,
    /// Additional parameters for the provider
    #[serde(default)]
    pub additional_params: HashMap<String, String>,
    /// Token limit, if set by the user
    pub token_limit: Option<usize>,
}

/// Default function for `use_emoji`
fn default_emoji() -> bool {
    true
}

// Default instruction preset to use
fn default_instruction_preset() -> String {
    "default".to_string()
}

impl Config {
    /// Load the configuration from git config
    pub fn load() -> Result<Self> {
        let mut config = Self::load_from_config("gitv");

        // Then try to load and merge project config if available
        if let Ok(project_config) = Self::load_project_config() {
            config.merge_with_project_config(project_config);
        }

        log_debug!("Configuration loaded: {config:?}");
        Ok(config)
    }

    /// Load configuration from git config
    fn load_from_config(prefix: &str) -> Self {
        let default_provider = Self::get_git_config_value(&format!("{prefix}.defaultprovider"))
            .unwrap_or("openai".to_string());
        let use_emoji = Self::get_git_config_bool(&format!("{prefix}.useemoji")).unwrap_or(true);
        let instructions =
            Self::get_git_config_value(&format!("{prefix}.instructions")).unwrap_or_default();
        let instruction_preset = Self::get_git_config_value(&format!("{prefix}.instructionpreset"))
            .unwrap_or("default".to_string());

        let mut providers = HashMap::new();
        // To load providers, we need to iterate over all keys with prefix
        // But git2 Config doesn't have easy way to iterate, so for now, assume known providers
        for provider in get_available_provider_names() {
            if let Some(api_key) =
                Self::get_git_config_value(&format!("{prefix}.{provider}-apikey"))
            {
                let default_model = get_default_model_for_provider(&provider).to_string();
                let model = Self::get_git_config_value(&format!("{prefix}.{provider}-model"))
                    .unwrap_or(default_model);
                let token_limit =
                    Self::get_git_config_i64(&format!("{prefix}.{provider}-tokenlimit")).map(|v| {
                        usize::try_from(v).expect("Failed to convert token limit from i64 to usize")
                    });
                let additional_params = HashMap::new();
                // For additional params, it's hard to iterate, so skip for now
                providers.insert(
                    provider.to_string(),
                    ProviderConfig {
                        api_key,
                        model,
                        additional_params,
                        token_limit,
                    },
                );
            }
        }

        Self {
            default_provider,
            providers,
            use_emoji: use_emoji,
            instructions,
            instruction_preset,
            temp_instructions: None,
            temp_preset: None,
            is_project_config: false,
        }
    }

    fn get_git_config_value(key: &str) -> Option<String> {
        let output = Command::new("git")
            .args(["config", "--get", key])
            .output()
            .ok()?;
        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }

    fn get_git_config_bool(key: &str) -> Option<bool> {
        Self::get_git_config_value(key).and_then(|v| v.parse().ok())
    }

    fn get_git_config_i64(key: &str) -> Option<i64> {
        Self::get_git_config_value(key).and_then(|v| v.parse().ok())
    }

    /// Load project-specific configuration
    pub fn load_project_config() -> Result<Self, anyhow::Error> {
        let mut project_config = Self::load_from_config("gitv");
        project_config.is_project_config = true;
        Ok(project_config)
    }

    /// Merge this config with project-specific config, with project config taking precedence
    /// But never allow API keys from project config
    pub fn merge_with_project_config(&mut self, project_config: Self) {
        log_debug!("Merging with project configuration");

        // Override default provider if set in project config
        if project_config.default_provider != Self::default().default_provider {
            self.default_provider = project_config.default_provider;
        }

        // Merge provider configs, but never allow API keys from project config
        for (provider, proj_provider_config) in project_config.providers {
            let entry = self.providers.entry(provider).or_default();

            // Don't override API keys from project config (security)
            if !proj_provider_config.model.is_empty() {
                entry.model = proj_provider_config.model;
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

        // Override other settings
        self.use_emoji = project_config.use_emoji;

        // Always override instructions field if set in project config
        self.instructions = project_config.instructions.clone();

        // Override preset
        if project_config.instruction_preset != default_instruction_preset() {
            self.instruction_preset = project_config.instruction_preset;
        }
    }

    /// Save the configuration to git config
    pub fn save(&self) -> Result<()> {
        // Don't save project configs to personal config file
        if self.is_project_config {
            return Ok(());
        }

        let mut config = GitConfig::open_default()?;
        self.save_to_config(&mut config, "gitv")?;
        log_debug!("Configuration saved to global git config: {self:?}");
        Ok(())
    }

    /// Save the configuration to a git config
    fn save_to_config(&self, config: &mut GitConfig, prefix: &str) -> Result<()> {
        // Set default provider
        config.set_str(&format!("{prefix}.defaultprovider"), &self.default_provider)?;

        // Set use emoji
        config.set_bool(&format!("{prefix}.useemoji"), self.use_emoji)?;

        // Set instructions
        config.set_str(&format!("{prefix}.instructions"), &self.instructions)?;

        // Set instruction preset
        config.set_str(
            &format!("{prefix}.instructionpreset"),
            &self.instruction_preset,
        )?;

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
                &provider_config.model,
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
    pub fn save_as_project_config(&self) -> Result<(), anyhow::Error> {
        let repo = git2::Repository::discover(".")?;

        // Before saving, create a copy that excludes API keys
        let mut project_config = self.clone();

        // Remove API keys from all providers
        for provider_config in project_config.providers.values_mut() {
            provider_config.api_key.clear();
        }

        // Mark as project config
        project_config.is_project_config = true;

        // Save to local git config
        let mut config = repo.config()?;
        project_config.save_to_config(&mut config, "gitv")?;
        log_debug!("Project configuration saved to local git config: {project_config:?}");
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

    pub fn set_temp_instructions(&mut self, instructions: Option<String>) {
        self.temp_instructions = instructions;
    }

    pub fn set_temp_preset(&mut self, preset: Option<String>) {
        self.temp_preset = preset;
    }

    pub fn get_effective_instructions(&self) -> String {
        let preset_library = get_instruction_preset_library();
        let preset_instructions = self
            .temp_preset
            .as_ref()
            .or(Some(&self.instruction_preset))
            .and_then(|p| preset_library.get_preset(p))
            .map(|p| p.instructions.clone())
            .unwrap_or_default();

        let custom_instructions = self
            .temp_instructions
            .as_ref()
            .unwrap_or(&self.instructions);

        format!("{preset_instructions}\n\n{custom_instructions}")
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
        use_emoji: Option<bool>,
        instructions: Option<String>,
        token_limit: Option<usize>,
    ) -> anyhow::Result<()> {
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
            provider_config.model = model;
        }
        if let Some(params) = additional_params {
            provider_config.additional_params.extend(params);
        }
        if let Some(emoji) = use_emoji {
            self.use_emoji = emoji;
        }
        if let Some(instr) = instructions {
            self.instructions = instr;
        }
        if let Some(limit) = token_limit {
            provider_config.token_limit = Some(limit);
        }

        log_debug!("Configuration updated: {self:?}");
        Ok(())
    }

    /// Get the configuration for a specific provider
    pub fn get_provider_config(&self, provider: &str) -> Option<&ProviderConfig> {
        // Special case: redirect "claude" to "anthropic"
        let provider_to_lookup = if provider.to_lowercase() == "claude" {
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
    pub fn set_project_config(&mut self, is_project: bool) {
        self.is_project_config = is_project;
    }

    /// Check if this is a project config
    pub fn is_project_config(&self) -> bool {
        self.is_project_config
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut providers = HashMap::new();
        for provider in get_available_provider_names() {
            providers.insert(provider.clone(), ProviderConfig::default_for(&provider));
        }

        // Default to OpenAI if available, otherwise use the first available provider
        let default_provider = if providers.contains_key("openai") {
            "openai".to_string()
        } else {
            providers.keys().next().map_or_else(
                || "openai".to_string(), // Fallback even if no providers (should never happen)
                std::clone::Clone::clone,
            )
        };

        Self {
            default_provider,
            providers,
            use_emoji: default_emoji(),
            instructions: String::new(),
            instruction_preset: default_instruction_preset(),
            temp_instructions: None,
            temp_preset: None,
            is_project_config: false,
        }
    }
}

impl ProviderConfig {
    /// Create a default provider configuration for a given provider
    pub fn default_for(provider: &str) -> Self {
        Self {
            api_key: String::new(),
            model: get_default_model_for_provider(provider).to_string(),
            additional_params: HashMap::new(),
            token_limit: None, // Will use the default from get_default_token_limit_for_provider
        }
    }

    /// Get the token limit for this provider configuration
    pub fn get_token_limit(&self) -> Option<usize> {
        self.token_limit
    }
}
