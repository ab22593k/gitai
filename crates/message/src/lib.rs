use anyhow::Result;
use clap::{Parser, crate_authors, crate_version};
use cloy::app::args::{self, MessageParams};
use cloy::commands::commit::service::CommitService;
use cloy::commands::commit::types::{GeneratedMessage, format_commit_message};
use cloy::commands::common::service::{create_commit_service, create_completion_service};
use cloy::commands::common::{run_with_spinner, validate_staged_files};
use cloy::common::CommonParams;
use cloy::config::Config;
use cloy::llm::messages;
use cloy::output;
use cloy::tui::run_tui_commit;

#[derive(Parser)]
#[command(
    name = "git-message",
    author = crate_authors!(),
    version = crate_version!(),
    about = "Generate a commit message using AI",
    after_help = args::get_dynamic_help(),
    styles = args::get_styles(),
)]
pub struct CommonArgs {
    #[command(flatten)]
    pub common: CommonParams,

    #[command(flatten)]
    pub params: MessageParams,
}

async fn generate_initial_message(
    service: &CommitService,
    instructions: &str,
) -> Result<GeneratedMessage> {
    let random_message = messages::get_waiting_message();
    let spinner = output::create_tui_spinner(&random_message.text);
    run_with_spinner(spinner, async || {
        service.generate_message(instructions).await
    })
    .await
}

pub struct MessageConfig {
    pub print: bool,
}

pub async fn handle_message_command(
    common: CommonParams,
    config: MessageConfig,
    repository_url: Option<String>,
) -> Result<()> {
    let print = config.print;
    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;

    let service = create_commit_service(&common, repository_url.clone(), &config).map_err(|e| {
        output::print_error(&format!("Error: {e}"));
        e
    })?;

    let completion_service =
        create_completion_service(&common, repository_url, &config).map_err(|e| {
            output::print_error(&format!("Error: {e}"));
            e
        })?;

    let git_info = service.get_git_info().await?;

    if git_info.staged_files.is_empty() {
        validate_staged_files(&git_info);
        return Ok(());
    }

    let effective_instructions = common
        .instructions
        .unwrap_or_else(|| config.instructions.clone());

    let initial_message = generate_initial_message(&service, &effective_instructions).await?;

    if print {
        println!("{}", format_commit_message(&initial_message));
        return Ok(());
    }

    if service.is_remote_repository() {
        output::print_warning(
            "Interactive commit not available for remote repositories. Using print mode instead.",
        );
        println!("{}", format_commit_message(&initial_message));
        return Ok(());
    }

    run_tui_commit(
        vec![initial_message],
        effective_instructions,
        service,
        completion_service,
        common.theme,
    )
    .await?;

    Ok(())
}

pub async fn handle_completion_command(
    common: CommonParams,
    prefix: String,
    context_ratio: Option<f32>,
    config: MessageConfig,
    repository_url: Option<String>,
) -> Result<()> {
    let print = config.print;

    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;

    let context_ratio = context_ratio.unwrap_or(0.5);

    if !(0.0..=1.0).contains(&context_ratio) {
        output::print_error("Context ratio must be between 0.0 and 1.0");
        return Err(anyhow::anyhow!("Invalid context ratio: {context_ratio}"));
    }

    output::print_info(&format!(
        "Completing message with prefix '{}' using {:.0}% context ratio",
        prefix,
        context_ratio * 100.0
    ));

    let service = create_completion_service(&common, repository_url, &config).map_err(|e| {
        output::print_error(&format!("Error: {e}"));
        output::print_info("\nPlease ensure the following:");
        output::print_info("1. Git is installed and accessible from the command line.");
        output::print_info(
            "2. You are running this command from within a Git repository or provide a repository URL with --repo.",
        );
        e
    })?;

    let git_info = service.get_git_info().await?;

    if git_info.staged_files.is_empty() {
        output::print_warning(
            "No staged changes. Please stage your changes before completing a commit message.",
        );
        output::print_info("You can stage changes using 'git add <file>' or 'git add .'");
        return Ok(());
    }

    let random_message = messages::get_waiting_message();
    let spinner = output::create_tui_spinner(&format!(
        "{} - Completing commit message",
        random_message.text
    ));

    let completed_message = run_with_spinner(spinner, async || {
        service.complete_message(&prefix, context_ratio).await
    })
    .await?;

    if print {
        println!("{}", format_commit_message(&completed_message));
        return Ok(());
    }

    if service.is_remote_repository() {
        output::print_warning(
            "Completion not available for remote repositories. Using print mode instead.",
        );
        println!("{}", format_commit_message(&completed_message));
        return Ok(());
    }

    output::print_info(&format!("Prefix: {prefix}"));
    output::print_info("Completed message:");
    println!("{}", format_commit_message(&completed_message));

    output::print_info(
        "\nUse --print to output only the completed message, or --auto-commit to commit directly.",
    );

    Ok(())
}

#[derive(Clone, Debug)]
pub struct MessageArgs {
    pub complete: bool,
    pub prefix: Option<String>,
    pub context_ratio: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct CmsgConfig {
    pub print_only: bool,
}

pub async fn handle_message(
    common: CommonParams,
    config: CmsgConfig,
    repository_url: Option<String>,
    args: MessageArgs,
) -> Result<()> {
    log::debug!(
        "Handling 'message' command with common: {common:?}, print: {}, complete: {}, prefix: {:?}, context_ratio: {:?}",
        config.print_only,
        args.complete,
        args.prefix,
        args.context_ratio,
    );

    if args.complete {
        let prefix_text = args
            .prefix
            .ok_or_else(|| anyhow::anyhow!("Prefix is required for completion mode"))?;

        handle_completion_command(
            common,
            prefix_text,
            args.context_ratio,
            MessageConfig {
                print: config.print_only,
            },
            repository_url,
        )
        .await
    } else {
        handle_message_command(
            common,
            MessageConfig {
                print: config.print_only,
            },
            repository_url,
        )
        .await
    }
}
