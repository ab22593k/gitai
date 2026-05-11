use anyhow::Result;
use clap::Parser;
use claw_core::{init_app, output::print_error};
use cloy_message::{CmsgConfig, CommonArgs, MessageArgs, handle_message};

#[tokio::main]
async fn main() -> Result<()> {
    init_app();

    let cli_args = CommonArgs::parse();
    let CommonArgs { mut common, params } = cli_args;
    let repository_url = std::mem::take(&mut common.repository_url);

    if let Err(e) = handle_message(
        common,
        CmsgConfig {
            print_only: params.print,
        },
        repository_url,
        MessageArgs {
            complete: params.complete,
            prefix: params.prefix,
            context_ratio: params.context_ratio,
        },
    )
    .await
    {
        print_error(&format!("Error: {e}"));
        std::process::exit(1);
    }

    Ok(())
}
