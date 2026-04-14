use anyhow::Result;
use clap::{Parser, crate_authors, crate_version};
use gitai::{
    app::{
        args::{self, CmsgConfig, MessageArgs, MessageParams},
        handlers,
    },
    common::CommonParams,
    init_app,
    output::print_error,
};

#[derive(Parser)]
#[command(
    name = "git-message",
    author = crate_authors!(),
    version = crate_version!(),
    about = "Generate a commit message using AI",
    after_help = args::get_dynamic_help(),
    styles = args::get_styles(),
)]
struct CliArgs {
    #[command(flatten)]
    common: CommonParams,

    #[command(flatten)]
    params: MessageParams,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_app();

    let args = CliArgs::parse();
    let repository_url = args.common.repository_url.clone();

    if let Err(e) = handlers::handle_message(
        args.common,
        CmsgConfig {
            print_only: args.params.print,
            dry: args.params.dry,
        },
        repository_url,
        MessageArgs {
            complete: args.params.complete,
            prefix: args.params.prefix,
            context_ratio: args.params.context_ratio,
        },
    )
    .await
    {
        print_error(&format!("Error: {e}"));
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        CliArgs::command().debug_assert();
    }
}
