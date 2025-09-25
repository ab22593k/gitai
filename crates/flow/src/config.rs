use crate::git::GitRepo;
use crate::instruction_presets::get_instruction_preset_library;
use crate::llm::{
    get_available_provider_names, get_default_model_for_provider, provider_requires_api_key,
};
use crate::log_debug;

use anyhow::{Context, Result, anyhow};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Configuration structure for the Git-Iris application
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    /// Default LLM provider
    pub default_provider: String,
    /// Provider-specific configurations
    pub providers: HashMap<String, ProviderConfig>,
    /// Flag indicating whether to use Gitmoji
    #[serde(default = "default_gitmoji")]
    pub use_gitmoji: bool,
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

/// Default function for `use_gitmoji`
fn default_gitmoji() -> bool {
    true
}

// Default instruction preset to use
fn default_instruction_preset() -> String {
    "default".to_string()
}

/// Project configuration filename
pub const PROJECT_CONFIG_FILENAME: &str = ".irisconfig";

impl Config {
    /// Load the configuration from the file
    pub fn load() -> Result<Self> {
        // First load personal config
        let config_path = Self::get_config_path()?;
        let mut config = if config_path.exists() {
            let config_content = fs::read_to_string(&config_path)?;
            let config: Self = toml::from_str(&config_content)?;
            Self::migrate_if_needed(config)
        } else {
            Self::default()
        };

        // Then try to load and merge project config if available
        if let Ok(project_config) = Self::load_project_config() {
            config.merge_with_project_config(project_config);
        }

        log_debug!("Configuration loaded: {:?}", config);
        Ok(config)
    }

    /// Load project-specific configuration
    pub fn load_project_config() -> Result<Self, anyhow::Error> {
        let config_path = Self::get_project_config_path()?;
        if !config_path.exists() {
            return Err(anyhow::anyhow!("Project configuration file not found"));
        }

        // Read the config file with improved error handling
        let config_str = match fs::read_to_string(&config_path) {
            Ok(content) => content,
            Err(e) => return Err(anyhow::anyhow!("Failed to read project config file: {e}")),
        };

        // Parse the TOML with improved error handling
        let mut config: Self = match toml::from_str(&config_str) {
            Ok(config) => config,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Invalid project configuration file format: {e}. Please check your {PROJECT_CONFIG_FILENAME} file for syntax errors."
                ));
            }
        };

        config.is_project_config = true;
        Ok(config)
    }

    /// Get the path to the project configuration file
    pub fn get_project_config_path() -> Result<PathBuf, anyhow::Error> {
        // Use the static method to get repo root
        let repo_root = crate::git::GitRepo::get_repo_root()?;
        Ok(repo_root.join(PROJECT_CONFIG_FILENAME))
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
        self.use_gitmoji = project_config.use_gitmoji;

        // Always override instructions field if set in project config
        self.instructions = project_config.instructions.clone();

        // Override preset
        if project_config.instruction_preset != default_instruction_preset() {
            self.instruction_preset = project_config.instruction_preset;
        }
    }

    /// Migrate older config formats if needed
    fn migrate_if_needed(mut config: Self) -> Self {
        // Migration: rename "claude" provider to "anthropic" if it exists
        let mut migration_performed = false;
        if config.providers.contains_key("claude") {
            log_debug!("Migrating 'claude' provider to 'anthropic'");
            if let Some(claude_config) = config.providers.remove("claude") {
                config
                    .providers
                    .insert("anthropic".to_string(), claude_config);
            }

            // Update default provider if it was set to claude
            if config.default_provider == "claude" {
                config.default_provider = "anthropic".to_string();
            }

            migration_performed = true;
        }

        // Save the config if a migration was performed
        if migration_performed {
            log_debug!("Saving configuration after migration");
            if let Err(e) = config.save() {
                log_debug!("Failed to save migrated config: {}", e);
            }
        }

        config
    }

    /// Save the configuration to the file
    pub fn save(&self) -> Result<()> {
        // Don't save project configs to personal config file
        if self.is_project_config {
            return Ok(());
        }

        let config_path = Self::get_config_path()?;
        let config_content = toml::to_string(self)?;
        fs::write(config_path, config_content)?;
        log_debug!("Configuration saved: {:?}", self);
        Ok(())
    }

    /// Save the configuration as a project-specific configuration
    pub fn save_as_project_config(&self) -> Result<(), anyhow::Error> {
        let config_path = Self::get_project_config_path()?;

        // Before saving, create a copy that excludes API keys
        let mut project_config = self.clone();

        // Remove API keys from all providers
        for provider_config in project_config.providers.values_mut() {
            provider_config.api_key.clear();
        }

        // Mark as project config
        project_config.is_project_config = true;

        // Convert to TOML string
        let config_str = toml::to_string_pretty(&project_config)?;

        // Write to file
        fs::write(config_path, config_str)?;

        Ok(())
    }

    /// Get the path to the configuration file
    fn get_config_path() -> Result<PathBuf> {
        let mut path =
            config_dir().ok_or_else(|| anyhow!("Unable to determine config directory"))?;
        path.push("git-iris");
        std::fs::create_dir_all(&path)?;
        path.push("config.toml");
        Ok(path)
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
        use_gitmoji: Option<bool>,
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
        if let Some(gitmoji) = use_gitmoji {
            self.use_gitmoji = gitmoji;
        }
        if let Some(instr) = instructions {
            self.instructions = instr;
        }
        if let Some(limit) = token_limit {
            provider_config.token_limit = Some(limit);
        }

        log_debug!("Configuration updated: {:?}", self);
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
            use_gitmoji: default_gitmoji(),
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
