use anyhow::Result;
use clap::Parser;
use gitpilot::{app, logger};

#[derive(Parser)]
#[command(
    name = "serve",
    about = "Start an MCP server to provide functionality to AI tools"
)]
struct ServeArgs {
    /// Enable development mode with more verbose logging
    #[arg(long, help = "Enable development mode with more verbose logging")]
    dev: bool,

    /// Transport type to use (stdio, sse)
    #[arg(
        short,
        long,
        help = "Transport type to use (stdio, sse)",
        default_value = "stdio"
    )]
    transport: String,

    /// Port to use for network transports
    #[arg(short, long, help = "Port to use for network transports")]
    port: Option<u16>,

    /// Listen address for network transports
    #[arg(
        long,
        help = "Listen address for network transports (e.g., '127.0.0.1', '0.0.0.0')",
        default_value = "127.0.0.1"
    )]
    listen_address: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    logger::init().expect("Failed to initialize logger");

    let args = ServeArgs::parse();
    match app::handle_serve_command(args.dev, args.transport, args.port, args.listen_address).await
    {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
