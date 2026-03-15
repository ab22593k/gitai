use anyhow::Result;
use clap::{Parser, crate_authors, crate_version};
use gitai::{
    app::{self, ChangelogParams},
    common::CommonParams,
    init_logger, init_tracing_to_file,
    ui::print_error,
};

#[derive(Parser)]
#[command(
    name = "git-changelog",
    author = crate_authors!(),
    version = crate_version!(),
    about = "Generate a changelog",
    after_help = app::get_dynamic_help(),
    styles = app::get_styles(),
)]
struct ChangelogArgs {
    #[command(flatten)]
    common: CommonParams,

    #[command(flatten)]
    params: ChangelogParams,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    init_tracing_to_file();

    let args = ChangelogArgs::parse();

    let repository_url = args.common.repository_url.clone();

    if let Err(e) = app::handle_changelog(
        args.common,
        args.params.from,
        args.params.to,
        repository_url,
        args.params.update,
        args.params.save,
        args.params.file,
        args.params.version_name,
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
        ChangelogArgs::command().debug_assert();
    }
}
