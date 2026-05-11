use crate::common::CommonParams;
use crate::sync::common::Method;
use clap::builder::{Styles, styling::AnsiColor};
use clap::{Args, Parser, Subcommand, crate_version};
use colored::Colorize;

#[derive(Parser)]
#[command(
    author,
    version = crate_version!(),
    about = "AI-powered Git workflow assistant",
    disable_version_flag = true,
    after_help = get_dynamic_help(),
    styles = get_styles(),
)]
pub struct App {
    #[command(subcommand)]
    pub command: Option<Gitai>,

    #[arg(
        short = 'l',
        long = "log",
        global = true,
        help = "Log debug messages to a file"
    )]
    pub log: bool,

    #[arg(
        long = "log-file",
        global = true,
        help = "Specify a custom log file path"
    )]
    pub log_file: Option<String>,

    #[arg(
        short = 'q',
        long = "quiet",
        global = true,
        help = "Suppress non-essential output"
    )]
    pub quiet: bool,

    #[arg(
        short = 'v',
        long = "version",
        global = true,
        help = "Display the version"
    )]
    pub version: bool,

    #[arg(
        short = 'r',
        long = "repo",
        global = true,
        help = "Repository URL to use instead of local repository"
    )]
    pub repository_url: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct MessageParams {
    #[arg(short, long, help = "Print the generated message to stdout and exit")]
    pub print: bool,

    #[arg(
        long,
        help = "Complete a commit message instead of generating from scratch"
    )]
    pub complete: bool,

    #[arg(
        long,
        help = "Prefix text to complete (required when using --complete)",
        requires = "complete"
    )]
    pub prefix: Option<String>,

    #[arg(
        long,
        help = "Context ratio for completion (0.0 to 1.0, default: 0.5)",
        requires = "complete",
        value_parser = parse_context_ratio
    )]
    pub context_ratio: Option<f32>,
}

#[derive(Args, Clone, Debug)]
pub struct PrParams {
    #[arg(long, help = "Starting branch, commit, or commitish for comparison")]
    pub from: Option<String>,

    #[arg(long, help = "Target branch, commit, or commitish for comparison")]
    pub to: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct ChangelogParams {
    #[arg(long)]
    pub from: Option<String>,

    #[arg(long)]
    pub to: Option<String>,

    #[arg(long, help = "Update the changelog file with the new changes")]
    pub update: bool,

    #[arg(long, help = "Auto-detect starting point and save to CHANGELOG.md")]
    pub save: bool,

    #[arg(long, help = "Path to the changelog file (defaults to CHANGELOG.md)")]
    pub file: Option<String>,

    #[arg(long, help = "Explicit version name to use in the changelog")]
    pub version_name: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct WireArgs {
    #[command(subcommand)]
    pub command: WireCommand,

    #[arg(global = true, short, long)]
    pub name: Option<String>,

    #[arg(global = true, short, long)]
    pub target: Option<String>,

    #[arg(global = true, short, long)]
    pub singlethread: bool,
}

#[derive(Subcommand, Clone, Debug)]
pub enum WireCommand {
    Sync {
        #[command(flatten)]
        source: WireSource,

        #[arg(long)]
        save: bool,

        #[arg(long)]
        no_save: bool,

        #[arg(long)]
        append: bool,

        #[arg(long)]
        global: bool,
    },
    Check {
        #[command(flatten)]
        source: WireSource,

        #[arg(long)]
        save: bool,

        #[arg(long)]
        no_save: bool,

        #[arg(long, requires = "save")]
        append: bool,

        #[arg(long)]
        global: bool,
    },
}

#[derive(clap::Args, Clone, Debug)]
pub struct WireSource {
    #[arg(long)]
    pub url: Option<String>,

    #[arg(long)]
    pub rev: Option<String>,

    #[arg(long, num_args = 1..)]
    pub src: Vec<String>,

    #[arg(long)]
    pub dst: Option<String>,

    #[arg(long)]
    pub entry_name: Option<String>,

    #[arg(long)]
    pub description: Option<String>,

    #[arg(long, value_enum)]
    pub method: Option<Method>,
}

#[derive(Subcommand)]
#[command(subcommand_negates_reqs = true)]
#[command(subcommand_precedence_over_arg = true)]
pub enum Gitai {
    Pr {
        #[command(flatten)]
        common: CommonParams,

        #[command(flatten)]
        params: PrParams,
    },
    Changelog {
        #[command(flatten)]
        common: CommonParams,

        #[command(flatten)]
        params: ChangelogParams,
    },
    Wire(WireArgs),
}

pub fn get_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Magenta.on_default().bold())
        .usage(AnsiColor::Cyan.on_default().bold())
        .literal(AnsiColor::Green.on_default().bold())
        .placeholder(AnsiColor::Yellow.on_default())
        .valid(AnsiColor::Blue.on_default().bold())
        .invalid(AnsiColor::Red.on_default().bold())
        .error(AnsiColor::Red.on_default().bold())
}

#[must_use]
pub fn parse_args() -> App {
    App::parse()
}

#[must_use]
pub fn get_dynamic_help() -> String {
    let mut providers = crate::llm::engine::get_available_provider_names();
    providers.sort();

    let providers_list = providers
        .iter()
        .map(|p| format!("{}", (*p).bold()))
        .collect::<Vec<_>>()
        .join(" • ");

    format!("\nAvailable LLM Providers: {providers_list}")
}

#[derive(Clone, Debug)]
pub struct ChangelogArgs {
    pub from: Option<String>,
    pub to: Option<String>,
    pub repository_url: Option<String>,
    pub update: bool,
    pub save: bool,
    pub file: Option<String>,
    pub version_name: Option<String>,
}

fn parse_context_ratio(s: &str) -> Result<f32, String> {
    let val: f32 = s
        .parse()
        .map_err(|_| format!("'{s}' is not a valid number"))?;
    if !(0.0..=1.0).contains(&val) {
        return Err(format!(
            "context ratio must be between 0.0 and 1.0, got {val}"
        ));
    }
    Ok(val)
}
