use anyhow::Result;
use clap::Parser;
use gitpilot::{commands::handle_presets_command, logger};

#[derive(Parser)]
#[command(
    name = "git-flow-list-presets",
    about = "List available instruction presets"
)]
struct ListPresetsArgs {}

#[tokio::main]
async fn main() -> Result<()> {
    logger::init().expect("Failed to initialize logger");

    let _args = ListPresetsArgs::parse();

    match handle_presets_command() {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
