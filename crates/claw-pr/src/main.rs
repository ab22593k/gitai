use anyhow::Result;
use clap::{Parser, crate_authors, crate_version};
use claw_core::{
    app::{
        args::{self, PrParams},
        handlers,
    },
    common::CommonParams,
    init_app,
    output::print_error,
};

#[derive(Parser)]
#[command(
    name = "git-pr",
    author = crate_authors!(),
    version = crate_version!(),
    about = "Generate a pull request description using AI",
    after_help = args::get_dynamic_help(),
    styles = args::get_styles(),
)]
struct PrArgs {
    #[command(flatten)]
    common: CommonParams,

    #[command(flatten)]
    params: PrParams,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_app();

    let cli_args = PrArgs::parse();
    let PrArgs { mut common, params } = cli_args;
    let repository_url = std::mem::take(&mut common.repository_url);

    if let Err(e) =
        handlers::handle_pr_command(common, params.from, params.to, repository_url).await
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
        PrArgs::command().debug_assert();
    }
}
