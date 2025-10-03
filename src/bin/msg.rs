use anyhow::Result;
use clap::Parser;
use gitpilot::cli::{self, CmsgConfig};
use gitpilot::common::CommonParams;
use gitpilot::logger;

#[derive(Parser)]
#[command(name = "git-flow-msg", about = "Generate a commit message using AI")]
#[allow(clippy::struct_excessive_bools)]
struct MsgArgs {
    #[command(flatten)]
    common: CommonParams,

    /// Automatically commit with the generated message
    #[arg(short, long, help = "Automatically commit with the generated message")]
    auto_commit: bool,

    /// Disable emoji for this commit
    #[arg(long, help = "Disable emoji for this commit")]
    no_emoji: bool,

    /// Print the generated message to stdout and exit
    #[arg(short, long, help = "Print the generated message to stdout and exit")]
    print: bool,

    /// Skip the verification step (pre/post commit hooks)
    #[arg(long, help = "Skip verification steps (pre/post commit hooks)")]
    no_verify: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    logger::init().expect("Failed to initialize logger");

    let args = MsgArgs::parse();

    let repository_url = args.common.repository_url.clone();

    match cli::handle_message(
        args.common,
        CmsgConfig {
            auto_commit: args.auto_commit,
            use_emoji: !args.no_emoji,
            print_only: args.print,
            verify: !args.no_verify,
        },
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
