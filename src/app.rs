use crate::common::CommonParams;
use crate::core::llm::get_available_provider_names;
use crate::features::changelog::{handle_changelog_command, handle_release_notes_command};
use crate::features::commit;
use clap::builder::{Styles, styling::AnsiColor};
use clap::{Parser, Subcommand, crate_version};
use colored::Colorize;
use log::debug;

/// CLI structure defining the available commands and global arguments
#[derive(Parser)]
#[command(
    author,
    version = crate_version!(),
    about = "Gait: AI-powered Git workflow assistant",
    disable_version_flag = true,
    after_help = get_dynamic_help(),
    styles = get_styles(),
)]
pub struct Cli {
    /// Subcommands available for the CLI
    #[command(subcommand)]
    pub command: Option<Gait>,

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

/// Enumeration of available subcommands
#[derive(Subcommand)]
#[command(subcommand_negates_reqs = true)]
#[command(subcommand_precedence_over_arg = true)]
pub enum Gait {
    // Feature commands first
    /// Generate a commit message using AI
    #[command(
        about = "Generate a commit message using AI",
        long_about = "Generate a commit message using AI based on the current Git context.",
        after_help = get_dynamic_help()
    )]
    Message {
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

        /// Print the generated PR description to stdout and exit
        #[arg(
            short,
            long,
            help = "Print the generated PR description to stdout and exit"
        )]
        print: bool,

        /// Starting branch, commit, or commitish for comparison
        #[arg(
            long,
            help = "Starting branch, commit, or commitish for comparison. For single commit analysis, specify just this parameter with a commit hash (e.g., --from abc1234). For reviewing multiple commits, use commitish syntax (e.g., --from HEAD~3 to review last 3 commits)"
        )]
        from: Option<String>,

        /// Target branch, commit, or commitish for comparison
        #[arg(
            long,
            help = "Target branch, commit, or commitish for comparison. For single commit analysis, specify just this parameter with a commit hash or commitish (e.g., --to HEAD~2)"
        )]
        to: Option<String>,
    },

    /// Generate a changelog
    #[command(
        about = "Generate a changelog",
        long_about = "Generate a changelog between two specified Git references."
    )]
    Changelog {
        #[command(flatten)]
        common: CommonParams,

        /// Starting Git reference (commit hash, tag, or branch name)
        #[arg(long, required = true)]
        from: String,

        /// Ending Git reference (commit hash, tag, or branch name). Defaults to HEAD if not specified.
        #[arg(long)]
        to: Option<String>,

        /// Update the changelog file with the new changes
        #[arg(long, help = "Update the changelog file with the new changes")]
        update: bool,

        /// Path to the changelog file
        #[arg(long, help = "Path to the changelog file (defaults to CHANGELOG.md)")]
        file: Option<String>,

        /// Explicit version name to use in the changelog instead of getting it from Git
        #[arg(long, help = "Explicit version name to use in the changelog")]
        version_name: Option<String>,
    },

    /// Generate release notes
    #[command(
        about = "Generate release notes",
        long_about = "Generate comprehensive release notes between two specified Git references."
    )]
    ReleaseNotes {
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
    },
}

/// Define custom styles for Clap
fn get_styles() -> Styles {
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
fn get_dynamic_help() -> String {
    let mut providers = get_available_provider_names();
    providers.sort();

    let providers_list = providers
        .iter()
        .map(|p| format!("{}", p.bold()))
        .collect::<Vec<_>>()
        .join(" • ");

    format!(
        "\\
Available LLM Providers: {providers_list}"
    )
}

/// Configuration for the cmsg command
#[allow(clippy::struct_excessive_bools)]
pub struct CmsgConfig {
    pub auto_commit: bool,
    pub print_only: bool,
    pub verify: bool,
    pub dry_run: bool,
    pub amend: bool,
    pub commit_ref: Option<String>,
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
        "Handling 'message' command with common: {common:?}, auto_commit: {}, print: {}, verify: {}, amend: {}, commit_ref: {:?}, complete: {complete}, prefix: {prefix:?}, context_ratio: {context_ratio:?}",
        config.auto_commit, config.print_only, config.verify, config.amend, config.commit_ref,
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
            config.verify,
            config.dry_run,
            config.amend,
            config.commit_ref,
            repository_url,
        )
        .await
    } else {
        // Handle generation mode
        commit::handle_message_command(
            common,
            config.auto_commit,
            config.print_only,
            config.verify,
            config.dry_run,
            config.amend,
            config.commit_ref,
            repository_url,
        )
        .await
    }
}

/// Handle the `Changelog` command
pub async fn handle_changelog(
    common: CommonParams,
    from: String,
    to: Option<String>,
    repository_url: Option<String>,
    update: bool,
    file: Option<String>,
    version_name: Option<String>,
) -> anyhow::Result<()> {
    debug!(
        "Handling 'changelog' command with common: {common:?}, from: {from}, to: {to:?}, update: {update}, file: {file:?}, version_name: {version_name:?}"
    );
    handle_changelog_command(common, from, to, repository_url, update, file, version_name).await
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
pub async fn handle_command(command: Gait, repository_url: Option<String>) -> anyhow::Result<()> {
    // Initialize tracing to file
    crate::core::llm::init_tracing_to_file();

    match command {
        Gait::Message {
            common,
            auto_commit,
            print,
            no_verify,
            amend,
            commit,
        } => {
            handle_message(
                common,
                CmsgConfig {
                    auto_commit,
                    print_only: print,
                    verify: !no_verify,
                    dry_run: false,
                    amend,
                    commit_ref: commit,
                },
                repository_url,
                false,
                None,
                None,
            )
            .await
        }
        Gait::Changelog {
            common,
            from,
            to,
            update,
            file,
            version_name,
        } => handle_changelog(common, from, to, repository_url, update, file, version_name).await,
        Gait::ReleaseNotes {
            common,
            from,
            to,
            version_name,
        } => handle_release_notes(common, from, to, repository_url, version_name).await,
        Gait::Pr {
            common,
            print,
            from,
            to,
        } => handle_pr_command(common, print, from, to, repository_url).await,
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
