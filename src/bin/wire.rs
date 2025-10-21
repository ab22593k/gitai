use clap::{Parser, Subcommand};
use colored::Colorize;
use gitai::remote::check;
use gitai::remote::common::Parsed;
use gitai::remote::common::Target;
use gitai::remote::common::sequence;
use gitai::remote::sync;
use std::process::exit;

pub use gitai::CachedRepository;
pub use gitai::RepositoryConfiguration;
pub use gitai::WireOperation;
use gitai::init_logger;

#[derive(Parser)]
#[command(version, author, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Narrow down the scope of commands targets by its name
    #[arg(global = true, short, long)]
    name: Option<String>,

    /// Narrow down the scope of commands targets by its name (same as `-n` and `--name`)
    #[arg(global = true, short, long)]
    target: Option<String>,

    /// Execute the command with single thread
    /// (slow, easy-to-read output, low resource consumption)
    #[arg(global = true, short, long)]
    singlethread: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Synchronizes code depending on a file '.gitwire' definition.
    Sync,

    /// Checks if the synchronized code identical to the original.
    Check,

    /// Directly synchronizes code depending on given arguments
    DirectSync {
        #[arg(long)]
        url: String,
        #[arg(long)]
        rev: String,
        #[arg(long)]
        src: String,
        #[arg(long)]
        dst: String,
    },

    /// Directly checks if the code is identical to the code led by given arguments.
    DirectCheck {
        #[arg(long)]
        url: String,
        #[arg(long)]
        rev: String,
        #[arg(long)]
        src: String,
        #[arg(long)]
        dst: String,
    },
}

#[tokio::main]
async fn main() {
    init_logger();

    let cli = Cli::parse();

    let target = cli.target.or(cli.name);

    let mode = if cli.singlethread {
        sequence::Mode::Single
    } else {
        sequence::Mode::Parallel
    };

    let result = match cli.command {
        Command::Sync => sync::sync_with_caching(&Target::Declared(target), mode).await,
        Command::Check => check::check(Target::Declared(target), &mode),
        Command::DirectSync { url, rev, src, dst } => sync::sync_with_caching(
            // Also use caching for direct sync
            &Target::Direct(Parsed {
                name: None,
                dsc: None,
                mtd: None,
                url,
                rev,
                src,
                dst,
            }),
            mode,
        ).await,
        Command::DirectCheck { url, rev, src, dst } => check::check(
            Target::Direct(Parsed {
                name: None,
                dsc: None,
                mtd: None,
                url,
                rev,
                src,
                dst,
            }),
            &mode,
        ),
    };

    match result.as_ref() {
        Ok(true) => println!("{}", "Success".green().bold()),
        Ok(false) => println!("{}", "Failure".red().bold()),
        Err(e) => eprintln!("{}", e.to_string().red().bold()),
    }

    match result {
        Ok(true) => exit(0),
        _ => exit(1),
    }
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}
