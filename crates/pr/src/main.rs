use anyhow::Result;
use clap::{Args, Parser, crate_authors, crate_version};
use cloy::{
    app::args::{get_dynamic_help, get_styles},
    common::CommonParams,
    init_app,
    output::print_error,
};
use cloy_pr::handle_pr_command;

#[derive(Args, Clone, Debug)]
struct PrParams {
    #[arg(long, help = "Starting branch, commit, or commitish for comparison")]
    from: Option<String>,

    #[arg(long, help = "Target branch, commit, or commitish for comparison")]
    to: Option<String>,
}

#[derive(Parser)]
#[command(
    name = "git-pr",
    author = crate_authors!(),
    version = crate_version!(),
    about = "Generate a pull request description using AI",
    after_help = get_dynamic_help(),
    styles = get_styles(),
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
    let PrArgs { mut common, params } = args;
    let repository_url = std::mem::take(&mut common.repository_url);

    if let Err(e) = handle_pr_command(common, params.from, params.to, repository_url).await {
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
