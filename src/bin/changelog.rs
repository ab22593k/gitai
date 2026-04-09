use anyhow::Result;
use clap::{Parser, crate_authors, crate_version};
use gitai::{
    app::{
        args::{self, ChangelogArgs, ChangelogParams},
        handlers,
    },
    common::CommonParams,
    init_app,
    ui::print_error,
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

    let args = CliArgs::parse();
    let repository_url = args.common.repository_url.clone();

    if let Err(e) = handlers::handle_changelog(
        args.common,
        ChangelogArgs {
            from: args.params.from,
            to: args.params.to,
            repository_url,
            update: args.params.update,
            save: args.params.save,
            file: args.params.file,
            version_name: args.params.version_name,
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
