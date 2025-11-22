use anyhow::Result;
use clap::Parser;
use gait::{app, common::CommonParams};

#[derive(Parser)]
#[command(name = "git-flow-release-notes", about = "Generate release notes")]
struct ReleaseNotesArgs {
    #[command(flatten)]
    common: CommonParams,

    /// Starting Git reference (commit hash, tag, or branch name)
    #[arg(long, required = true)]
    from: String,

    /// Ending Git reference (commit hash, tag, or branch name). Defaults to HEAD if not specified.
    #[arg(long)]
    to: Option<String>,

    /// Explicit version name to use in the release notes instead of getting it from Git
    #[arg(long, help = "Explicit version name to use in the release notes")]
    version_name: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = ReleaseNotesArgs::parse();

    let repository_url = args.common.repository_url.clone();

    match app::handle_release_notes(
        args.common,
        args.from,
        args.to,
        repository_url,
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
