use anyhow::Result;
use clap::Parser;
use gitpilot::{app, common::CommonParams, logger};

#[derive(Parser)]
#[command(name = "git-flow-review", about = "Review staged changes using AI")]
struct ReviewArgs {
    #[command(flatten)]
    common: CommonParams,

    /// Print the generated review to stdout and exit
    #[arg(short, long, help = "Print the generated review to stdout and exit")]
    print: bool,

    /// Include unstaged changes in the review
    #[arg(long, help = "Include unstaged changes in the review")]
    include_unstaged: bool,

    /// Review a specific commit by ID (hash, branch, or reference)
    #[arg(
        long,
        help = "Review a specific commit by ID (hash, branch, or reference)"
    )]
    commit: Option<String>,

    /// Starting branch for comparison (defaults to 'main')
    #[arg(
        long,
        help = "Starting branch for comparison (defaults to 'main'). Used with --to for branch comparison reviews"
    )]
    from: Option<String>,

    /// Target branch for comparison (e.g., 'feature-branch', 'pr-branch')
    #[arg(
        long,
        help = "Target branch for comparison (e.g., 'feature-branch', 'pr-branch'). Used with --from for branch comparison reviews"
    )]
    to: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    logger::init().expect("Failed to initialize logger");

    let args = ReviewArgs::parse();
    let repository_url = args.common.repository_url.clone();

    match app::handle_review(
        args.common,
        args.print,
        repository_url,
        args.include_unstaged,
        args.commit,
        args.from,
        args.to,
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
