use clap::{Parser, Subcommand};
use colored::Colorize;
use gitai::{
    init_logger,
    remote::{
        check,
        common::{Method, Parsed, Target, TargetConfig, sequence},
        sync,
    },
};
use std::process::exit;

pub use gitai::{CachedRepository, RepositoryConfiguration, WireOperation};

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
    /// Execute the command with single thread (slow, easy-to-read output, low resource consumption)
    #[arg(global = true, short, long)]
    singlethread: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Synchronizes code depending on a file '.gitwire.toml' definition or CLI arguments.
    Sync {
        /// Repository URL
        #[arg(long)]
        url: Option<String>,

        /// Git revision (branch, tag, or commit hash)
        #[arg(long)]
        rev: Option<String>,

        /// Source path(s) in the repository. Can be:
        /// - Single value: --src "lib"
        /// - Multiple flags: --src lib --src tools
        /// - JSON array: `--src '["lib", "tools", "src"]'`
        #[arg(long, num_args = 1..)]
        src: Vec<String>,

        /// Destination path in the local repository
        #[arg(long)]
        dst: Option<String>,

        /// Optional name for this wire entry
        #[arg(long)]
        entry_name: Option<String>,

        /// Optional description for this wire entry
        #[arg(long)]
        description: Option<String>,

        /// Clone method (shallow, `shallow_no_sparse`, or partial)
        #[arg(long, value_parser = ["shallow", "shallow_no_sparse", "partial"])]
        method: Option<String>,

        /// Save this configuration to .gitwire.toml after syncing
        #[arg(long)]
        save: bool,

        /// Append to existing .gitwire.toml instead of creating new (requires --save)
        #[arg(long, requires = "save")]
        append: bool,
    },

    /// Checks if the synchronized code identical to the original.
    Check {
        /// Repository URL
        #[arg(long)]
        url: Option<String>,

        /// Git revision (branch, tag, or commit hash)
        #[arg(long)]
        rev: Option<String>,

        /// Source path(s) in the repository
        #[arg(long, num_args = 1..)]
        src: Vec<String>,

        /// Destination path in the local repository
        #[arg(long)]
        dst: Option<String>,

        /// Optional name for this wire entry
        #[arg(long)]
        entry_name: Option<String>,

        /// Optional description for this wire entry
        #[arg(long)]
        description: Option<String>,

        /// Clone method (shallow, `shallow_no_sparse`, or partial)
        #[arg(long, value_parser = ["shallow", "shallow_no_sparse", "partial"])]
        method: Option<String>,

        /// Save this configuration to .gitwire.toml after checking
        #[arg(long)]
        save: bool,

        /// Append to existing .gitwire.toml instead of creating new (requires --save)
        #[arg(long, requires = "save")]
        append: bool,
    },
}

/// Build a Parsed struct from CLI arguments
/// Returns None if no CLI args are provided, Some(Parsed) otherwise
fn build_parsed_from_cli(
    url: Option<String>,
    rev: Option<String>,
    src: Vec<String>,
    dst: Option<String>,
    entry_name: Option<String>,
    description: Option<String>,
    method: Option<&String>,
) -> Option<Parsed> {
    // If no required fields are provided, return None
    if url.is_none() && rev.is_none() && src.is_empty() && dst.is_none() {
        return None;
    }

    // Parse src - handle JSON array if provided as single argument
    let src_paths = if src.len() == 1 && src[0].trim().starts_with('[') {
        // Try to parse as JSON array
        serde_json::from_str::<Vec<String>>(&src[0]).unwrap_or(src)
    } else {
        src
    };

    // Parse method
    let mtd = method.and_then(|m| match m.as_str() {
        "shallow" => Some(Method::Shallow),
        "shallow_no_sparse" => Some(Method::ShallowNoSparse),
        "partial" => Some(Method::Partial),
        _ => None,
    });

    Some(Parsed {
        name: entry_name,
        dsc: description,
        url: url.unwrap_or_default(),
        rev: rev.unwrap_or_default(),
        src: src_paths,
        dst: dst.unwrap_or_default(),
        mtd,
    })
}

#[tokio::main]
async fn main() {
    init_logger();

    let cli = Cli::parse();

    let target_name = cli.target.or(cli.name);

    let mode = if cli.singlethread {
        sequence::Mode::Single
    } else {
        sequence::Mode::Parallel
    };

    let result = match cli.command {
        Command::Sync {
            url,
            rev,
            src,
            dst,
            entry_name,
            description,
            method,
            save,
            append,
        } => {
            let cli_override =
                build_parsed_from_cli(url, rev, src, dst, entry_name, description, method.as_ref());

            // Validate CLI override if provided
            if let Some(ref parsed) = cli_override
                && let Err(e) = parsed.validate()
            {
                eprintln!("{}", format!("Invalid arguments: {e}").red().bold());
                exit(1);
            }

            let target_config = TargetConfig {
                name_filter: target_name,
                cli_override,
                save_config: save,
                append_config: append,
            };

            sync::sync_with_caching(&Target::Declared(target_config), mode).await
        }

        Command::Check {
            url,
            rev,
            src,
            dst,
            entry_name,
            description,
            method,
            save,
            append,
        } => {
            let cli_override =
                build_parsed_from_cli(url, rev, src, dst, entry_name, description, method.as_ref());

            // Validate CLI override if provided
            if let Some(ref parsed) = cli_override
                && let Err(e) = parsed.validate()
            {
                eprintln!("{}", format!("Invalid arguments: {e}").red().bold());
                exit(1);
            }

            let target_config = TargetConfig {
                name_filter: target_name,
                cli_override,
                save_config: save,
                append_config: append,
            };

            check::check(&Target::Declared(target_config), &mode)
        }
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
