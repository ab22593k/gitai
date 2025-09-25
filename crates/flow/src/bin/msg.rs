use anyhow::Result;
use clap::Parser;
use gwtflow::cli::{self, CmsgConfig};
use gwtflow::common::CommonParams;

#[derive(Parser)]
#[command(name = "git-flow-msg", about = "Generate a commit message using AI")]
#[allow(clippy::struct_excessive_bools)]
struct MsgArgs {
    #[command(flatten)]
    common: CommonParams,

    /// Automatically commit with the generated message
    #[arg(short, long, help = "Automatically commit with the generated message")]
    auto_commit: bool,

    /// Disable Gitmoji for this commit
    #[arg(long, help = "Disable Gitmoji for this commit")]
    no_gitmoji: bool,

    /// Print the generated message to stdout and exit
    #[arg(short, long, help = "Print the generated message to stdout and exit")]
    print: bool,

    /// Skip the verification step (pre/post commit hooks)
    #[arg(long, help = "Skip verification steps (pre/post commit hooks)")]
    no_verify: bool,

    /// Repository URL to use instead of local repository
    #[arg(
        short = 'r',
        long = "repo",
        help = "Repository URL to use instead of local repository"
    )]
    repository_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    gwtflow::logger::init().expect("Failed to initialize logger");
    
    let args = MsgArgs::parse();
    
    match cli::handle_cmsg(
        args.common,
        CmsgConfig {
            auto_commit: args.auto_commit,
            use_gitmoji: !args.no_gitmoji,
            print_only: args.print,
            verify: !args.no_verify,
        },
        args.repository_url,
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