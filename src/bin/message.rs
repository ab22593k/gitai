use anyhow::Result;
use clap::Parser;
use gait::{
    app::{self, CmsgConfig},
    common::CommonParams,
};

#[derive(Parser)]
#[command(name = "git-message", about = "Generate a commit message using AI")]
#[allow(clippy::struct_excessive_bools)]
struct MessageArgs {
    #[command(flatten)]
    common: CommonParams,

    /// Automatically commit with the generated message
    #[arg(short, long, help = "Automatically commit with the generated message")]
    auto_commit: bool,

    /// Print the generated message to stdout and exit
    #[arg(short, long, help = "Print the generated message to stdout and exit")]
    print: bool,

    /// Skip the verification step (pre/post commit hooks)
    #[arg(long, help = "Skip verification steps (pre/post commit hooks)")]
    no_verify: bool,

    /// Dry run mode: do not make real HTTP requests, for UI testing
    #[arg(
        long,
        help = "Dry run mode: do not make real HTTP requests, for UI testing"
    )]
    dry_run: bool,

    /// Amend the last commit or a specific commit with a new AI-generated message
    #[arg(
        long,
        help = "Amend the last commit or a specific commit with a new AI-generated message"
    )]
    amend: bool,

    /// Specific commit to amend (hash, branch, or reference). Defaults to HEAD when --amend is used
    #[arg(
        long,
        help = "Specific commit to amend (hash, branch, or reference). Defaults to HEAD when --amend is used"
    )]
    commit: Option<String>,

    /// Complete a commit message instead of generating from scratch
    #[arg(
        long,
        help = "Complete a commit message instead of generating from scratch"
    )]
    complete: bool,

    /// Prefix text to complete (required when using --complete)
    #[arg(
        long,
        help = "Prefix text to complete (required when using --complete)",
        requires = "complete"
    )]
    prefix: Option<String>,

    /// Context ratio for completion (0.0 to 1.0, default: 0.5)
    #[arg(
        long,
        help = "Context ratio for completion (0.0 to 1.0, default: 0.5). Higher values use more of the original message as context.",
        requires = "complete"
    )]
    context_ratio: Option<f32>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = MessageArgs::parse();
    let repository_url = args.common.repository_url.clone();

    match app::handle_message(
        args.common,
        CmsgConfig {
            auto_commit: args.auto_commit,
            print_only: args.print,
            verify: !args.no_verify,
            dry_run: args.dry_run,
            amend: args.amend,
            commit_ref: args.commit,
        },
        repository_url,
        args.complete,
        args.prefix,
        args.context_ratio,
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
