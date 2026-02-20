use crate::config::Config;
use crate::core::llm::get_available_provider_names;
use anyhow::Result;
use clap::{Args, ValueEnum};
use std::env;
use std::fmt::Write;
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DetailLevel {
    Minimal,
    Standard,
    Detailed,
}

impl FromStr for DetailLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "minimal" => Ok(Self::Minimal),
            "standard" => Ok(Self::Standard),
            "detailed" => Ok(Self::Detailed),
            _ => Err(anyhow::anyhow!("Invalid detail level: {s}")),
        }
    }
}

impl DetailLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Standard => "standard",
            Self::Detailed => "detailed",
        }
    }
}

/// Theme mode (Dark, Light, System)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
    System,
}

impl ThemeMode {
    #[must_use]
    pub fn resolve(&self) -> Self {
        match self {
            Self::System => Self::detect(),
            _ => *self,
        }
    }

    fn detect() -> Self {
        // Heuristic using COLORFGBG (common in xterm-based terminals)
        if let Ok(val) = env::var("COLORFGBG") {
            let parts: Vec<&str> = val.split(';').collect();
            // Format is usually "fg;bg" or just "bg" depending on implementation
            // Standard ANSI colors: 0-7 (dark/normal), 8-15 (bright)
            // 0=Black, 7=White, 15=Bright White.
            if parts.len() == 2
                && let Ok(bg) = parts[1].parse::<u8>()
            {
                // If background is white-ish (7 or 15) or high index, assume light
                if bg == 7 || bg == 15 || bg > 10 {
                    return Self::Light;
                }
            }
        }

        // Check standard ENVIRONMENT variables that might indicate light mode
        if let Ok(mode) = env::var("GTK_THEME")
            && mode.to_lowercase().contains("light")
        {
            return Self::Light;
        }

        Self::Dark
    }
}

#[derive(Args, Clone, Debug)]
pub struct CommonParams {
    /// Override default LLM provider
    #[arg(long, help = "Override default LLM provider", value_parser = available_providers_parser)]
    pub provider: Option<String>,

    /// Override default LLM model
    #[arg(long, help = "Override default LLM model")]
    pub model: Option<String>,

    /// Custom instructions for this operation
    #[arg(short, long, help = "Custom instructions for this operation")]
    pub instructions: Option<String>,

    /// Set the detail level
    #[arg(
        long,
        help = "Set the detail level (minimal, standard, detailed)",
        default_value = "standard"
    )]
    pub detail_level: String,

    /// Repository URL to use instead of local repository
    #[arg(
        short = 'r',
        long = "repo",
        help = "Repository URL to use instead of local repository"
    )]
    pub repository_url: Option<String>,

    /// Theme mode (dark, light, system)
    #[arg(
        long = "theme",
        help = "Theme mode (dark, light, system)",
        default_value = "dark"
    )]
    pub theme: ThemeMode,
}

impl Default for CommonParams {
    fn default() -> Self {
        Self {
            provider: None,
            model: None,
            instructions: None,
            detail_level: "standard".to_string(),
            repository_url: None,
            theme: ThemeMode::Dark,
        }
    }
}

impl CommonParams {
    pub fn apply_to_config(&self, config: &mut Config) -> Result<bool> {
        let mut changes_made = false;

        if let Some(provider) = &self.provider {
            let provider_name = provider.to_lowercase();

            // Check if we need to update the default provider
            if config.default_provider != provider_name {
                // Ensure the provider exists in the providers HashMap
                if !config.providers.contains_key(&provider_name) {
                    // Import ProviderConfig here
                    use crate::config::ProviderConfig;
                    config.providers.insert(
                        provider_name.clone(),
                        ProviderConfig::default_for(&provider_name),
                    );
                }

                config.default_provider.clone_from(&provider_name);
                changes_made = true;
            }
        }

        if let Some(model) = &self.model {
            let provider_name = config.default_provider.clone();
            // Ensure the provider exists in the providers HashMap
            if !config.providers.contains_key(&provider_name) {
                use crate::config::ProviderConfig;
                config.providers.insert(
                    provider_name.clone(),
                    ProviderConfig::default_for(&provider_name),
                );
            }

            if let Some(provider_config) = config.providers.get_mut(&provider_name)
                && provider_config.model_name != *model
            {
                provider_config.model_name.clone_from(model);
                changes_made = true;
            }
        }

        if let Some(instructions) = &self.instructions {
            config.set_temp_instructions(Some(instructions.clone()));
            // Note: temp instructions don't count as permanent changes
        }

        Ok(changes_made)
    }
}

/// Validates that a provider name is available in the system
pub fn available_providers_parser(s: &str) -> Result<String, String> {
    let provider_name = s.to_lowercase();
    let available_providers = get_available_provider_names();

    if available_providers
        .iter()
        .any(|p| p.to_lowercase() == provider_name)
    {
        Ok(provider_name)
    } else {
        Err(format!(
            "Invalid provider '{}'. Available providers: {}",
            s,
            available_providers.join(", ")
        ))
    }
}

pub fn get_combined_instructions(config: &Config) -> String {
    let mut prompt = String::from("\n\n");

    if !config.instructions.is_empty() {
        write!(
            &mut prompt,
            "\n\nAdditional instructions for the request:\n{}\n\n",
            config.get_effective_instructions()
        )
        .expect("write to string should not fail");
    }

    prompt
}
