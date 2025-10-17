use anyhow::Result;
use clap::Parser;
use gitai::{app, common::CommonParams, logger};

#[derive(Parser)]
#[command(name = "git-rebase", about = "Interactive rebase with AI assistance")]
struct RebaseArgs {
    #[command(flatten)]
    common: CommonParams,

    /// Upstream branch/commit to rebase onto
    #[arg(required = true, help = "Upstream branch/commit to rebase onto")]
    upstream: String,

    /// Branch to rebase (defaults to current branch)
    #[arg(short, long, help = "Branch to rebase (defaults to current branch)")]
    branch: Option<String>,

    /// Auto-apply AI suggestions without interactive prompt
    #[arg(long, help = "Auto-apply AI suggestions without interactive prompt")]
    auto_apply: bool,

    /// Focus on specific commit types (feat, fix, refactor, etc.)
    #[arg(
        long,
        help = "Focus on specific commit types (comma-separated: feat,fix,refactor,etc.)"
    )]
    commit_types: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    logger::init().expect("Failed to initialize logger");

    let args = RebaseArgs::parse();
    let repository_url = args.common.repository_url.clone();

    match app::handle_rebase_command(
        args.common,
        args.upstream,
        args.branch,
        args.auto_apply,
        args.commit_types,
        repository_url,
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