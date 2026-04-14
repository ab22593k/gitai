use anyhow::Result;
use clap::{Parser, crate_authors, crate_version};
use gitai::{
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

    let args = PrArgs::parse();
    let repository_url = args.common.repository_url.clone();

    if let Err(e) = handlers::handle_pr_command(
        args.common,
        args.params.print,
        args.params.from,
        args.params.to,
        repository_url,
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
        PrArgs::command().debug_assert();
    }
}
