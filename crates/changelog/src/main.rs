use anyhow::Result;
use changelog::{ChangelogCommandConfig, handle_changelog_command};
use clap::{Args, Parser, crate_authors, crate_version};
use cloy::{
    app::args::{get_dynamic_help, get_styles},
    common::CommonParams,
    init_app,
    output::print_error,
};

#[derive(Args, Clone, Debug)]
struct ChangelogParams {
    #[arg(long)]
    from: Option<String>,

    #[arg(long)]
    to: Option<String>,

    #[arg(long, help = "Update the changelog file with the new changes")]
    update: bool,

    #[arg(long, help = "Auto-detect starting point and save to CHANGELOG.md")]
    save: bool,

    #[arg(long, help = "Path to the changelog file (defaults to CHANGELOG.md)")]
    file: Option<String>,

    #[arg(long, help = "Explicit version name to use in the changelog")]
    version_name: Option<String>,
}

#[derive(Parser)]
#[command(
    name = "git-changelog",
    author = crate_authors!(),
    version = crate_version!(),
    about = "Generate a changelog",
    after_help = get_dynamic_help(),
    styles = get_styles(),
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

    if let Err(e) = handle_changelog_command(
        common,
        ChangelogCommandConfig {
            from: params.from,
            to: params.to,
            repository_url,
            update_file: params.update,
            save: params.save,
            changelog_path: params.file,
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
