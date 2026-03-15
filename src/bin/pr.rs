use anyhow::Result;
use clap::{Parser, crate_authors, crate_version};
use gitai::{
    app::{self, PrParams},
    common::CommonParams,
    init_logger, init_tracing_to_file,
    ui::print_error,
};

#[derive(Parser)]
#[command(
    name = "git-pr",
    author = crate_authors!(),
    version = crate_version!(),
    about = "Generate a pull request description using AI",
    after_help = app::get_dynamic_help(),
    styles = app::get_styles(),
)]
struct PrArgs {
    #[command(flatten)]
    common: CommonParams,

    #[command(flatten)]
    params: PrParams,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    init_tracing_to_file();

    let args = PrArgs::parse();

    let repository_url = args.common.repository_url.clone();

    if let Err(e) = app::handle_pr_command(
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
