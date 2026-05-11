use anyhow::Result;
use clap::{Parser, crate_authors, crate_version};
use claw_core::{
    app::{
        args::{self, ChangelogArgs, ChangelogParams},
        handlers,
    },
    common::CommonParams,
    init_app,
    output::print_error,
};

#[derive(Parser)]
#[command(
    name = "git-changelog",
    author = crate_authors!(),
    version = crate_version!(),
    about = "Generate a changelog",
    after_help = args::get_dynamic_help(),
    styles = args::get_styles(),
)]
struct CliArgs {
    #[command(flatten)]
    common: CommonParams,

    #[command(flatten)]
    params: ChangelogParams,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_app();

    let cli_args = CliArgs::parse();
    let CliArgs { mut common, params } = cli_args;
    let repository_url = std::mem::take(&mut common.repository_url);

    if let Err(e) = handlers::handle_changelog(
        common,
        ChangelogArgs {
            from: params.from,
            to: params.to,
            repository_url,
            update: params.update,
            save: params.save,
            file: params.file,
            version_name: params.version_name,
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
