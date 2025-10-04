use crate::instruction_presets::{
    PresetType, get_instruction_preset_library, list_presets_formatted_by_type,
};
use crate::log_debug;
use crate::mcp::config::{MCPServerConfig, MCPTransportType};
use crate::mcp::server;
use anyhow::Result;
use colored::Colorize;

/// Handle the 'presets' command
pub fn handle_presets_command() -> Result<()> {
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
