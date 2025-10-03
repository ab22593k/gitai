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

    // Apply emoji setting
    if let Some(use_emoji) = common.emoji
        && config.use_emoji != use_emoji
    {
        config.use_emoji = use_emoji;
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

    Ok(())
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
        "\ngitpilot Instruction Presets\n".bright_magenta().bold()
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
    println!("  gitpilot message --preset <preset-key>");
    println!("  gitpilot review --preset <preset-key>");
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
