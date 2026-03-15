use crate::common::CommonParams;
use crate::core::llm::get_available_provider_names;
use crate::features::changelog::{handle_changelog_command, handle_release_notes_command};
use crate::features::commit;
use clap::builder::{Styles, styling::AnsiColor};
use clap::{Args, Parser, Subcommand, crate_version};
use colored::Colorize;
use log::debug;

/// CLI structure defining the available commands and global arguments
#[derive(Parser)]
#[command(
    author,
    version = crate_version!(),
    about = "GitAI: AI-powered Git workflow assistant",
    disable_version_flag = true,
    after_help = get_dynamic_help(),
    styles = get_styles(),
)]
pub struct Cli {
    /// Subcommands available for the CLI
    #[command(subcommand)]
    pub command: Option<Gitai>,

    /// Log debug messages to a file
    #[arg(
        short = 'l',
        long = "log",
        global = true,
        help = "Log debug messages to a file"
    )]
    pub log: bool,

    /// Specify a custom log file path
    #[arg(
        long = "log-file",
        global = true,
        help = "Specify a custom log file path"
    )]
    pub log_file: Option<String>,

    /// Suppress non-essential output (spinners, waiting messages, etc.)
    #[arg(
        short = 'q',
        long = "quiet",
        global = true,
        help = "Suppress non-essential output"
    )]
    pub quiet: bool,

    /// Display the version
    #[arg(
        short = 'v',
        long = "version",
        global = true,
        help = "Display the version"
    )]
    pub version: bool,

    /// Repository URL to use instead of local repository
    #[arg(
        short = 'r',
        long = "repo",
        global = true,
        help = "Repository URL to use instead of local repository"
    )]
    pub repository_url: Option<String>,
}

/// Arguments for the message generation command
#[derive(Args, Clone, Debug)]
pub struct MessageParams {
    /// Print the generated message to stdout and exit
    #[arg(short, long, help = "Print the generated message to stdout and exit")]
    pub print: bool,

    /// Dry run mode: do not make real HTTP requests, for UI testing
    #[arg(
        long,
        help = "Dry run mode: do not make real HTTP requests, for UI testing"
    )]
    pub dry_run: bool,

    /// Complete a commit message instead of generating from scratch
    #[arg(
        long,
        help = "Complete a commit message instead of generating from scratch"
    )]
    pub complete: bool,

    /// Prefix text to complete (required when using --complete)
    #[arg(
        long,
        help = "Prefix text to complete (required when using --complete)",
        requires = "complete"
    )]
    pub prefix: Option<String>,

    /// Context ratio for completion (0.0 to 1.0, default: 0.5)
    #[arg(
        long,
        help = "Context ratio for completion (0.0 to 1.0, default: 0.5). Higher values use more of the original message as context.",
        requires = "complete"
    )]
    pub context_ratio: Option<f32>,
}

/// Arguments for the PR generation command
#[derive(Args, Clone, Debug)]
pub struct PrParams {
    /// Print the generated PR description to stdout and exit
    #[arg(
        short,
        long,
        help = "Print the generated PR description to stdout and exit"
    )]
    pub print: bool,

    /// Starting branch, commit, or commitish for comparison
    #[arg(
        long,
        help = "Starting branch, commit, or commitish for comparison. For single commit analysis, specify just this parameter with a commit hash (e.g., --from abc1234). For reviewing multiple commits, use commitish syntax (e.g., --from HEAD~3 to review last 3 commits)"
    )]
    pub from: Option<String>,

    /// Target branch, commit, or commitish for comparison
    #[arg(
        long,
        help = "Target branch, commit, or commitish for comparison. For single commit analysis, specify just this parameter with a commit hash or commitish (e.g., --to HEAD~2)"
    )]
    pub to: Option<String>,
}

/// Arguments for the changelog generation command
#[derive(Args, Clone, Debug)]
pub struct ChangelogParams {
    /// Starting Git reference (commit hash, tag, or branch name)
    /// If not specified, defaults to the latest tag (or first commit if no tags exist)
    /// when using --save
    #[arg(long)]
    pub from: Option<String>,

    /// Ending Git reference (commit hash, tag, or branch name). Defaults to HEAD if not specified.
    #[arg(long)]
    pub to: Option<String>,

    /// Update the changelog file with the new changes
    #[arg(long, help = "Update the changelog file with the new changes")]
    pub update: bool,

    /// Save: automatically detect the starting reference (latest tag or first commit)
    /// and update CHANGELOG.md
    #[arg(long, help = "Auto-detect starting point and save to CHANGELOG.md")]
    pub save: bool,

    /// Path to the changelog file
    #[arg(long, help = "Path to the changelog file (defaults to CHANGELOG.md)")]
    pub file: Option<String>,

    /// Explicit version name to use in the changelog instead of getting it from Git
    #[arg(long, help = "Explicit version name to use in the changelog")]
    pub version_name: Option<String>,
}

/// Arguments for the release notes generation command
#[derive(Args, Clone, Debug)]
pub struct ReleaseNotesParams {
    /// Starting Git reference (commit hash, tag, or branch name)
    #[arg(long, required = true)]
    pub from: String,

    /// Ending Git reference (commit hash, tag, or branch name). Defaults to HEAD if not specified.
    #[arg(long)]
    pub to: Option<String>,

    /// Explicit version name to use in the release notes instead of getting it from Git
    #[arg(long, help = "Explicit version name to use in the release notes")]
    pub version_name: Option<String>,
}

/// Enumeration of available subcommands
#[derive(Subcommand)]
#[command(subcommand_negates_reqs = true)]
#[command(subcommand_precedence_over_arg = true)]
pub enum Gitai {
    /// Generate a commit message using AI
    #[command(
        about = "Generate a commit message using AI",
        long_about = "Generate a commit message using AI based on the current Git context.",
        after_help = get_dynamic_help()
    )]
    Message {
        #[command(flatten)]
        common: CommonParams,

        #[command(flatten)]
        params: MessageParams,
    },

    /// Generate a pull request description
    #[command(
        about = "Generate a pull request description using AI",
        long_about = "Generate a comprehensive pull request description based on commit ranges, branch differences, or single commits. Analyzes the overall changeset as an atomic unit and creates professional PR descriptions with summaries, detailed explanations, and testing notes.\\
\\
Usage examples:\\
• Single commit: --from abc1234 or --to abc1234\\
• Single commitish: --from HEAD~1 or --to HEAD~2\\
• Multiple commits: --from HEAD~3 (reviews last 3 commits)\\
• Commit range: --from abc1234 --to def5678\\
• Branch comparison: --from main --to feature-branch\\
• From main to branch: --to feature-branch\\
\\
Supported commitish syntax: HEAD~2, HEAD^, @~3, main~1, origin/main^, etc."
    )]
    Pr {
        #[command(flatten)]
        common: CommonParams,

        #[command(flatten)]
        params: PrParams,
    },

    /// Generate a changelog
    #[command(
        about = "Generate a changelog",
        long_about = "Generate a changelog between two specified Git references."
    )]
    Changelog {
        #[command(flatten)]
        common: CommonParams,

        #[command(flatten)]
        params: ChangelogParams,
    },

    /// Generate release notes
    #[command(
        about = "Generate release notes",
        long_about = "Generate comprehensive release notes between two specified Git references."
    )]
    ReleaseNotes {
        #[command(flatten)]
        common: CommonParams,

        #[command(flatten)]
        params: ReleaseNotesParams,
    },
}

/// Define custom styles for Clap
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

/// Parse the command-line arguments
pub fn parse_args() -> Cli {
    Cli::parse()
}

/// Generate dynamic help including available LLM providers
pub fn get_dynamic_help() -> String {
    let mut providers = get_available_provider_names();
    providers.sort();

    let providers_list = providers
        .iter()
        .map(|p| format!("{}", p.bold()))
        .collect::<Vec<_>>()
        .join(" • ");

    format!("\nAvailable LLM Providers: {providers_list}")
}

/// Configuration for the cmsg command
#[allow(clippy::struct_excessive_bools)]
pub struct CmsgConfig {
    pub print_only: bool,
    pub dry_run: bool,
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_message(
    common: CommonParams,
    config: CmsgConfig,
    repository_url: Option<String>,
    complete: bool,
    prefix: Option<String>,
    context_ratio: Option<f32>,
) -> anyhow::Result<()> {
    debug!(
        "Handling 'message' command with common: {common:?}, print: {}, complete: {complete}, prefix: {prefix:?}, context_ratio: {context_ratio:?}",
        config.print_only,
    );

    if complete {
        // Handle completion mode
        let prefix_text =
            prefix.ok_or_else(|| anyhow::anyhow!("Prefix is required for completion mode"))?;
        let context_ratio_val = context_ratio.unwrap_or(0.5);

        commit::handle_completion_command(
            common,
            prefix_text,
            Some(context_ratio_val),
            config.print_only,
            config.dry_run,
            repository_url,
        )
        .await
    } else {
        // Handle generation mode
        commit::handle_message_command(common, config.print_only, config.dry_run, repository_url)
            .await
    }
}

/// Handle the `Changelog` command
#[allow(clippy::too_many_arguments)]
pub async fn handle_changelog(
    common: CommonParams,
    from: Option<String>,
    to: Option<String>,
    repository_url: Option<String>,
    update: bool,
    save: bool,
    file: Option<String>,
    version_name: Option<String>,
) -> anyhow::Result<()> {
    debug!(
        "Handling 'changelog' command with common: {common:?}, from: {from:?}, to: {to:?}, update: {update}, save: {save}, file: {file:?}, version_name: {version_name:?}"
    );
    handle_changelog_command(
        common,
        from,
        to,
        repository_url,
        update,
        save,
        file,
        version_name,
    )
    .await
}

/// Handle the `ReleaseNotes` command
pub async fn handle_release_notes(
    common: CommonParams,
    from: String,
    to: Option<String>,
    repository_url: Option<String>,
    version_name: Option<String>,
) -> anyhow::Result<()> {
    debug!(
        "Handling 'release-notes' command with common: {common:?}, from: {from}, to: {to:?}, version_name: {version_name:?}"
    );
    handle_release_notes_command(common, from, to, repository_url, version_name).await
}

/// Handle the command based on parsed arguments
pub async fn handle_command(command: Gitai, repository_url: Option<String>) -> anyhow::Result<()> {
    // Initialize tracing to file
    crate::core::llm::init_tracing_to_file();

    match command {
        Gitai::Message { common, params } => {
            handle_message(
                common,
                CmsgConfig {
                    print_only: params.print,
                    dry_run: params.dry_run,
                },
                repository_url,
                params.complete,
                params.prefix,
                params.context_ratio,
            )
            .await
        }
        Gitai::Changelog { common, params } => {
            handle_changelog(
                common,
                params.from,
                params.to,
                repository_url,
                params.update,
                params.save,
                params.file,
                params.version_name,
            )
            .await
        }
        Gitai::ReleaseNotes { common, params } => {
            handle_release_notes(
                common,
                params.from,
                params.to,
                repository_url,
                params.version_name,
            )
            .await
        }
        Gitai::Pr { common, params } => {
            handle_pr_command(common, params.print, params.from, params.to, repository_url).await
        }
    }
}

/// Handle the `Pr` command
pub async fn handle_pr_command(
    common: CommonParams,
    print: bool,
    from: Option<String>,
    to: Option<String>,
    repository_url: Option<String>,
) -> anyhow::Result<()> {
    debug!(
        "Handling 'pr' command with common: {common:?}, print: {print}, from: {from:?}, to: {to:?}"
    );
    commit::handle_pr_command(common, print, repository_url, from, to).await
}
