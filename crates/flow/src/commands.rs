use crate::ProviderConfig;
use crate::common::CommonParams;
use crate::config::Config;
use crate::instruction_presets::{
    PresetType, get_instruction_preset_library, list_presets_formatted_by_type,
};
use crate::llm::get_available_provider_names;
use crate::log_debug;
use crate::mcp::config::{MCPServerConfig, MCPTransportType};
use crate::mcp::server;
use crate::ui;
use anyhow::Context;
use anyhow::{Result, anyhow};
use colored::Colorize;
use std::collections::HashMap;

/// Apply common configuration changes to a config object
/// Returns true if any changes were made
///
/// This centralized function handles changes to configuration objects, used by both
/// personal and project configuration commands.
///
/// # Arguments
///
/// * `config` - The configuration object to modify
/// * `common` - Common parameters from command line
/// * `model` - Optional model to set for the selected provider
/// * `token_limit` - Optional token limit to set
/// * `param` - Optional additional parameters to set
/// * `api_key` - Optional API key to set (ignored in project configs)
///
/// # Returns
///
/// Boolean indicating if any changes were made to the configuration
fn apply_config_changes(
    config: &mut Config,
    common: &CommonParams,
    model: Option<String>,
    token_limit: Option<usize>,
    param: Option<Vec<String>>,
    api_key: Option<String>,
) -> anyhow::Result<bool> {
    let mut changes_made = false;

    // Apply common parameters to the config and track if changes were made
    let common_changes = common.apply_to_config(config)?;
    changes_made |= common_changes;

    // Handle provider change - but skip if already handled by apply_to_config
    if let Some(provider) = &common.provider {
        if !get_available_provider_names().iter().any(|p| p == provider) {
            return Err(anyhow!("Invalid provider: {provider}"));
        }
        // Only check for provider insertion if it wasn't already handled
        if !config.providers.contains_key(provider) {
            config
                .providers
                .insert(provider.clone(), ProviderConfig::default_for(provider));
            changes_made = true;
        }
    }

    let provider_config = config
        .providers
        .get_mut(&config.default_provider)
        .context("Could not get default provider")?;

    // Apply API key if provided
    if let Some(key) = api_key
        && provider_config.api_key != key
    {
        provider_config.api_key = key;
        changes_made = true;
    }

    // Apply model change
    if let Some(model) = model
        && provider_config.model != model
    {
        provider_config.model = model;
        changes_made = true;
    }

    // Apply parameter changes
    if let Some(params) = param {
        let additional_params = parse_additional_params(&params);
        if provider_config.additional_params != additional_params {
            provider_config.additional_params = additional_params;
            changes_made = true;
        }
    }

    // Apply gitmoji setting
    if let Some(use_gitmoji) = common.gitmoji
        && config.use_gitmoji != use_gitmoji
    {
        config.use_gitmoji = use_gitmoji;
        changes_made = true;
    }

    // Apply instructions
    if let Some(instr) = &common.instructions
        && config.instructions != *instr
    {
        config.instructions.clone_from(instr);
        changes_made = true;
    }

    // Apply token limit
    if let Some(limit) = token_limit
        && provider_config.token_limit != Some(limit)
    {
        provider_config.token_limit = Some(limit);
        changes_made = true;
    }

    // Apply preset
    if let Some(preset) = &common.preset {
        let preset_library = get_instruction_preset_library();
        if preset_library.get_preset(preset).is_some() {
            if config.instruction_preset != *preset {
                config.instruction_preset.clone_from(preset);
                changes_made = true;
            }
        } else {
            return Err(anyhow!("Invalid preset: {preset}"));
        }
    }

    Ok(changes_made)
}

/// Handle the 'config' command
#[allow(clippy::too_many_lines)]
pub fn handle_config_command(
    common: &CommonParams,
    api_key: Option<String>,
    model: Option<String>,
    token_limit: Option<usize>,
    param: Option<Vec<String>>,
) -> anyhow::Result<()> {
    log_debug!(
        "Starting 'config' command with common: {:?}, api_key: {:?}, model: {:?}, token_limit: {:?}, param: {:?}",
        common,
        api_key,
        model,
        token_limit,
        param
    );

    let mut config = Config::load()?;

    // Apply configuration changes
    let changes_made =
        apply_config_changes(&mut config, common, model, token_limit, param, api_key)?;

    if changes_made {
        config.save()?;
        ui::print_success("Configuration updated successfully.");
        ui::print_newline();
    }

    // Print the configuration with beautiful styling
    print_configuration(&config);

    Ok(())
}

/// Process and apply configuration changes to a config object for project configs
///
/// This is a specialized wrapper around the `apply_config_changes` function that ensures
/// API keys are never passed to project configuration files.
///
/// # Arguments
///
/// * `config` - The configuration object to modify
/// * `common` - Common parameters from command line
/// * `model` - Optional model to set for the selected provider
/// * `token_limit` - Optional token limit to set
/// * `param` - Optional additional parameters to set
///
/// # Returns
///
/// Boolean indicating if any changes were made to the configuration
fn apply_project_config_changes(
    config: &mut Config,
    common: &CommonParams,
    model: Option<String>,
    token_limit: Option<usize>,
    param: Option<Vec<String>>,
) -> anyhow::Result<bool> {
    // Use the shared function but don't pass an API key (never stored in project configs)
    apply_config_changes(config, common, model, token_limit, param, None)
}

/// Handle printing current project configuration
///
/// Loads and displays the current project configuration if it exists,
/// or shows a message if no project configuration is found.
fn print_project_config() {
    if let Ok(project_config) = Config::load_project_config() {
        ui::print_message(&format!(
            "\n{}",
            "Current project configuration:".bright_cyan().bold()
        ));
        print_configuration(&project_config);
    } else {
        ui::print_message(&format!(
            "\n{}",
            "No project configuration file found.".yellow()
        ));
        ui::print_message("You can create one with the project-config command.");
    }
}

/// Handle the 'project-config' command
///
/// Creates or updates a project-specific configuration file (.irisconfig)
/// in the repository root. Project configurations allow teams to share
/// common settings without sharing sensitive data like API keys.
///
/// # Security
///
/// API keys are never stored in project configuration files, ensuring that
/// sensitive credentials are not accidentally committed to version control.
///
/// # Arguments
///
/// * `common` - Common parameters from command line
/// * `model` - Optional model to set for the selected provider
/// * `token_limit` - Optional token limit to set
/// * `param` - Optional additional parameters to set
/// * `print` - Whether to just print the current project config
///
/// # Returns
///
/// Result indicating success or an error
pub fn handle_project_config_command(
    common: &CommonParams,
    model: Option<String>,
    token_limit: Option<usize>,
    param: Option<Vec<String>>,
    print: bool,
) -> anyhow::Result<()> {
    log_debug!(
        "Starting 'project-config' command with common: {:?}, model: {:?}, token_limit: {:?}, param: {:?}, print: {}",
        common,
        model,
        token_limit,
        param,
        print
    );

    // Load the global config first
    let mut config = Config::load()?;

    // Set up a header to explain what's happening
    println!("\n{}", "✨ Project Configuration".bright_magenta().bold());

    // If print-only mode, just display the current project config if it exists
    if print {
        print_project_config();
        return Ok(());
    }

    // Apply changes and track if any were made
    let changes_made =
        apply_project_config_changes(&mut config, common, model, token_limit, param)?;

    if changes_made {
        // Save to project config file
        config.save_as_project_config()?;
        ui::print_success("Project configuration created/updated successfully.");
        println!();

        // Print a notice about API keys not being stored in project config
        println!(
            "{}",
            "Note: API keys are never stored in project configuration files."
                .yellow()
                .italic()
        );
        println!();

        // Print the newly created/updated config
        println!("{}", "Current project configuration:".bright_cyan().bold());
        print_configuration(&config);
    } else {
        println!("{}", "No changes made to project configuration.".yellow());
        println!();

        // Check if a project config exists and show it if found
        if let Ok(project_config) = Config::load_project_config() {
            println!("{}", "Current project configuration:".bright_cyan().bold());
            print_configuration(&project_config);
        } else {
            println!("{}", "No project configuration exists yet.".bright_yellow());
            println!(
                "{}",
                "Use this command with options like --model or --provider to create one."
                    .bright_white()
            );
        }
    }

    Ok(())
}

/// Display the configuration with beautiful styling and colors
fn print_configuration(config: &Config) {
    // Create a title with gradient
    println!(
        "\n{}",
        ui::create_gradient_text("🔮 Git-Iris Configuration 🔮").bold()
    );
    println!();

    // Global settings section
    println!("{}", "Global Settings".bright_magenta().bold().underline());
    println!();

    let provider_label = "Default Provider:".bright_cyan().bold();
    let provider_value = config.default_provider.bright_white();
    println!("  {} {} {}", "🔹".cyan(), provider_label, provider_value);

    let gitmoji_label = "Use Gitmoji:".bright_cyan().bold();
    let gitmoji_value = if config.use_gitmoji {
        "Yes".bright_green()
    } else {
        "No".bright_red()
    };
    println!("  {} {} {}", "🔹".cyan(), gitmoji_label, gitmoji_value);

    let preset_label = "Instruction Preset:".bright_cyan().bold();
    let preset_value = config.instruction_preset.bright_yellow();
    println!("  {} {} {}", "🔹".cyan(), preset_label, preset_value);

    println!();

    // Instructions section (if any)
    if !config.instructions.is_empty() {
        println!("{}", "Custom Instructions".bright_blue().bold().underline());
        println!();

        // Display full instructions, preserving newlines
        config.instructions.lines().for_each(|line| {
            println!("  {}", line.bright_white().italic());
        });

        println!();
    }

    // Provider configurations
    for (provider, provider_config) in &config.providers {
        println!(
            "{}",
            format!("Provider: {provider}")
                .bright_green()
                .bold()
                .underline()
        );
        println!();

        // API Key status with lock emoji
        let api_key_label = "API Key:".yellow().bold();
        let api_key_value = if provider_config.api_key.is_empty() {
            "Not set".bright_red().italic()
        } else {
            "Set ✓".bright_green()
        };
        println!("  {} {} {}", "🔒".yellow(), api_key_label, api_key_value);

        // Model with sparkle emoji
        let model_label = "Model:".yellow().bold();
        let model_value = provider_config.model.bright_cyan();
        println!("  {} {} {}", "✨".yellow(), model_label, model_value);

        // Token limit with gauge emoji
        let token_limit_label = "Token Limit:".yellow().bold();
        let token_limit_value = provider_config
            .token_limit
            .map_or("Default".bright_yellow(), |limit| {
                limit.to_string().bright_white()
            });
        println!(
            "  {} {} {}",
            "🔢".yellow(),
            token_limit_label,
            token_limit_value
        );

        // Additional parameters if any
        if !provider_config.additional_params.is_empty() {
            let params_label = "Additional Parameters:".yellow().bold();
            println!("  {} {}", "🔧".yellow(), params_label);

            for (key, value) in &provider_config.additional_params {
                println!("    - {}: {}", key.bright_blue(), value.bright_white());
            }
        }

        println!();
    }
}

/// Parse additional parameters from the command line
fn parse_additional_params(params: &[String]) -> HashMap<String, String> {
    params
        .iter()
        .filter_map(|param| {
            let parts: Vec<&str> = param.splitn(2, '=').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect()
}

/// Handle the '`list_presets`' command
pub fn handle_list_presets_command() -> Result<()> {
    let library = get_instruction_preset_library();

    // Get different categories of presets
    let both_presets = list_presets_formatted_by_type(&library, Some(PresetType::Both));
    let commit_only_presets = list_presets_formatted_by_type(&library, Some(PresetType::Commit));
    let review_only_presets = list_presets_formatted_by_type(&library, Some(PresetType::Review));

    println!(
        "{}",
        "\nGit-Iris Instruction Presets\n".bright_magenta().bold()
    );

    println!(
        "{}",
        "General Presets (usable for both commit and review):"
            .bright_cyan()
            .bold()
    );
    println!("{both_presets}\n");

    if !commit_only_presets.is_empty() {
        println!("{}", "Commit-specific Presets:".bright_green().bold());
        println!("{commit_only_presets}\n");
    }

    if !review_only_presets.is_empty() {
        println!("{}", "Review-specific Presets:".bright_blue().bold());
        println!("{review_only_presets}\n");
    }

    println!("{}", "Usage:".bright_yellow().bold());
    println!("  git-iris cmsg --preset <preset-key>");
    println!("  git-iris review --preset <preset-key>");
    println!("\nPreset types: [B] = Both commands, [C] = Commit only, [R] = Review only");

    Ok(())
}

/// Handle the 'serve' command to start an MCP server
pub async fn handle_serve_command(
    dev: bool,
    transport: String,
    port: Option<u16>,
    listen_address: Option<String>,
) -> anyhow::Result<()> {
    log_debug!(
        "Starting 'serve' command with dev: {}, transport: {}, port: {:?}, listen_address: {:?}",
        dev,
        transport,
        port,
        listen_address
    );

    // Create MCP server configuration
    let mut config = MCPServerConfig::default();

    // Set development mode
    if dev {
        config = config.with_dev_mode();
    }

    // Set transport type
    let transport_type = match transport.to_lowercase().as_str() {
        "stdio" => MCPTransportType::StdIO,
        "sse" => MCPTransportType::SSE,
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid transport type: {transport}. Valid options are: stdio, sse"
            ));
        }
    };
    config = config.with_transport(transport_type);

    // Set port if provided
    if let Some(p) = port {
        config = config.with_port(p);
    }

    // Set listen address if provided
    if let Some(addr) = listen_address {
        config = config.with_listen_address(addr);
    }

    // Start the server - all UI output is now handled inside serve implementation
    server::serve(config).await
}
