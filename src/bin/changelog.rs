use anyhow::Result;
use clap::Parser;
use gitai::{app, common::CommonParams};

#[derive(Parser)]
#[command(name = "gitai-changelog", about = "Generate a changelog")]
struct ChangelogArgs {
    #[command(flatten)]
    common: CommonParams,
    /// Starting Git reference (commit hash, tag, or branch name)
    #[arg(long, required = true)]
    from: String,
    /// Ending Git reference (commit hash, tag, or branch name). Defaults to HEAD if not specified.
    #[arg(long)]
    to: Option<String>,
    /// Update the changelog file with the new changes
    #[arg(long, help = "Update the changelog file with the new changes")]
    update: bool,
    /// Path to the changelog file
    #[arg(long, help = "Path to the changelog file (defaults to CHANGELOG.md)")]
    file: Option<String>,
    /// Explicit version name to use in the changelog instead of getting it from Git
    #[arg(long, help = "Explicit version name to use in the changelog")]
    version_name: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = ChangelogArgs::parse();

    let repository_url = args.common.repository_url.clone();

    match app::handle_changelog(
        args.common,
        args.from,
        args.to,
        repository_url,
        args.update,
        args.file,
        args.version_name,
    )
    .await
    {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
