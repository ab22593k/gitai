use anyhow::Result;
use clap::{Parser, crate_authors, crate_version};
use gitai::{
    app::{self, CmsgConfig, MessageParams},
    common::CommonParams,
    init_app,
    ui::print_error,
};

#[derive(Parser)]
#[command(
    name = "git-message",
    author = crate_authors!(),
    version = crate_version!(),
    about = "Generate a commit message using AI",
    after_help = app::get_dynamic_help(),
    styles = app::get_styles(),
)]
struct MessageArgs {
    #[command(flatten)]
    common: CommonParams,

    #[command(flatten)]
    params: MessageParams,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_app();

    let args = MessageArgs::parse();
    let repository_url = args.common.repository_url.clone();

    if let Err(e) = app::handle_message(
        args.common,
        CmsgConfig {
            print_only: args.params.print,
            dry_run: args.params.dry_run,
        },
        repository_url,
        args.params.complete,
        args.params.prefix,
        args.params.context_ratio,
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
        MessageArgs::command().debug_assert();
    }
}
