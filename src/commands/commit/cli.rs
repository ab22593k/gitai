use super::service::CommitService;
use super::types::{format_commit_message, format_pull_request};
use crate::commands::commit::types;
use crate::commands::common::service::{create_commit_service, create_completion_service};
use crate::commands::common::{run_with_spinner, validate_staged_files};
use crate::common::CommonParams;
use crate::config::Config;
use crate::llm::messages;
use crate::tui::run_tui_commit;
use crate::tui::spinner::SpinnerState;
use crate::ui;

use anyhow::Result;
use std::sync::Arc;

async fn generate_initial_message(
    service: &CommitService,
    instructions: &str,
    dry: bool,
) -> Result<types::GeneratedMessage> {
    if dry {
        return Ok(types::GeneratedMessage {
            title: "Fix bug in UI rendering".to_string(),
            message: "Updated the layout to properly handle dynamic constraints and improve user experience.".to_string(),
        });
    }

    let random_message = messages::get_waiting_message();
    let spinner = ui::create_tui_spinner(&random_message.text);
    run_with_spinner(spinner, async || {
        service.generate_message(instructions).await
    })
    .await
}

pub async fn handle_message_command(
    common: CommonParams,
    config: MessageConfig,
    repository_url: Option<String>,
) -> Result<()> {
    let print = config.print;
    let dry = config.dry;
    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;

    let service = create_commit_service(&common, repository_url.clone(), &config).map_err(|e| {
        ui::print_error(&format!("Error: {e}"));
        e
    })?;

    let completion_service =
        create_completion_service(&common, repository_url, &config).map_err(|e| {
            ui::print_error(&format!("Error: {e}"));
            e
        })?;

    let git_info = service.get_git_info().await?;

    if git_info.staged_files.is_empty() && !dry {
        validate_staged_files(&git_info, dry);
        return Ok(());
    }

    let effective_instructions = common
        .instructions
        .unwrap_or_else(|| config.instructions.clone());

    let initial_message = generate_initial_message(&service, &effective_instructions, dry).await?;

    if print {
        println!("{}", format_commit_message(&initial_message));
        return Ok(());
    }

    if service.is_remote_repository() {
        ui::print_warning(
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

/// Handles the PR description generation command
pub async fn handle_pr_command(
    common: CommonParams,
    _print: bool,
    repository_url: Option<String>,
    from: Option<String>,
    to: Option<String>,
) -> Result<()> {
    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;

    // Setup the service
    let service = setup_pr_service(&common, repository_url, &config)?;

    // Generate the PR description
    let pr_description = generate_pr_based_on_parameters(service, common, config, from, to).await?;

    // Print the PR description to stdout
    println!("{}", format_pull_request(&pr_description));

    Ok(())
}

pub struct MessageConfig {
    pub print: bool,
    pub dry: bool,
}

/// Handles the commit message completion command
pub async fn handle_completion_command(
    common: CommonParams,
    prefix: String,
    context_ratio: Option<f32>,
    config: MessageConfig,
    repository_url: Option<String>,
) -> Result<()> {
    let print = config.print;
    let dry = config.dry;
    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;

    // Default context ratio to 0.5 (50%) if not specified
    let context_ratio = context_ratio.unwrap_or(0.5);

    // Validate context ratio
    if !(0.0..=1.0).contains(&context_ratio) {
        ui::print_error("Context ratio must be between 0.0 and 1.0");
        return Err(anyhow::anyhow!("Invalid context ratio: {context_ratio}"));
    }

    // Provide helpful information about context ratios
    ui::print_info(&format!(
        "Completing message with prefix '{}' using {:.0}% context ratio",
        prefix,
        context_ratio * 100.0
    ));

    // Create the completion service
    let service = create_completion_service(
        &common,
        repository_url,
        &config,
    ).map_err(|e| {
        ui::print_error(&format!("Error: {e}"));
        ui::print_info("\nPlease ensure the following:");
        ui::print_info("1. Git is installed and accessible from the command line.");
        ui::print_info(
            "2. You are running this command from within a Git repository or provide a repository URL with --repo.",
        );
        e
    })?;

    let git_info = service.get_git_info().await?;

    if git_info.staged_files.is_empty() && !dry {
        ui::print_warning(
            "No staged changes. Please stage your changes before completing a commit message.",
        );
        ui::print_info("You can stage changes using 'git add <file>' or 'git add .'");
        return Ok(());
    }

    let _effective_instructions = common
        .instructions
        .unwrap_or_else(|| config.instructions.clone());

    // Create spinner for completion generation
    let random_message = messages::get_waiting_message();
    let spinner = ui::create_tui_spinner(&format!(
        "{} - Completing commit message",
        random_message.text
    ));

    // Generate completion with spinner display
    let completed_message = if dry {
        types::GeneratedMessage {
            title: format!("{}: Complete the implementation", prefix),
            message: "Add comprehensive error handling and improve code documentation.".to_string(),
        }
    } else {
        run_with_spinner(spinner, async || {
            service.complete_message(&prefix, context_ratio).await
        })
        .await?
    };

    if print {
        println!("{}", format_commit_message(&completed_message));
        return Ok(());
    }

    // For completion, we don't support auto-commit since the user needs to review the completion
    if service.is_remote_repository() {
        ui::print_warning(
            "Completion not available for remote repositories. Using print mode instead.",
        );
        println!("{}", format_commit_message(&completed_message));
        return Ok(());
    }

    // Show the completed message and let user decide
    ui::print_info(&format!("Prefix: {}", prefix));
    ui::print_info("Completed message:");
    println!("{}", format_commit_message(&completed_message));

    ui::print_info(
        "\nUse --print to output only the completed message, or --auto-commit to commit directly.",
    );

    Ok(())
}

/// Sets up the PR service with proper configuration
fn setup_pr_service(
    common: &CommonParams,
    repository_url: Option<String>,
    config: &Config,
) -> Result<Arc<CommitService>> {
    // Use the common function for service creation
    create_commit_service(common, repository_url, config)
}

/// Generates a PR description based on the provided parameters
async fn generate_pr_based_on_parameters(
    service: Arc<CommitService>,
    common: CommonParams,
    config: Config,
    from: Option<String>,
    to: Option<String>,
) -> Result<super::types::GeneratedPullRequest> {
    let effective_instructions = common
        .instructions
        .unwrap_or_else(|| config.instructions.clone());

    // Create spinner for PR generation
    let random_message = messages::get_waiting_message();
    let spinner = ui::create_tui_spinner(
        format!("{} - Generating PR description", random_message.text).as_str(),
    );

    // Generate PR description with spinner display
    let pr_description = run_with_spinner(spinner, async || match (from, to) {
        (Some(from_ref), Some(to_ref)) => {
            handle_from_and_to_parameters(
                service,
                &effective_instructions,
                from_ref,
                to_ref,
                random_message,
            )
            .await
        }
        (None, Some(to_ref)) => {
            handle_to_only_parameter(service, &effective_instructions, to_ref, random_message).await
        }
        (Some(from_ref), None) => {
            handle_from_only_parameter(service, &effective_instructions, from_ref, random_message)
                .await
        }
        (None, None) => {
            handle_no_parameters(service, &effective_instructions, random_message).await
        }
    })
    .await?;

    Ok(pr_description)
}

/// Handle case where both --from and --to parameters are provided
async fn handle_from_and_to_parameters(
    service: Arc<CommitService>,
    effective_instructions: &str,
    from_ref: String,
    to_ref: String,
    random_message: &messages::ColoredMessage,
) -> Result<super::types::GeneratedPullRequest> {
    // Special case: if from and to are the same, treat as single commit analysis
    if from_ref == to_ref {
        ui::create_tui_spinner(
            format!(
                "{} - Analyzing single commit: {}",
                random_message.text, from_ref
            )
            .as_str(),
        )
        .tick();

        service
            .generate_pr_for_commit_range(
                effective_instructions,
                &format!("{from_ref}^"),
                &from_ref,
            )
            .await
    } else if is_likely_commit_hash_or_commitish(&from_ref)
        || is_likely_commit_hash_or_commitish(&to_ref)
    {
        // Check if these look like commit hashes (7+ hex chars) or branches
        // Treat as commit range
        ui::create_tui_spinner(
            format!(
                "{} - Analyzing commit range: {}..{}",
                random_message.text, from_ref, to_ref
            )
            .as_str(),
        )
        .tick();

        service
            .generate_pr_for_commit_range(effective_instructions, &from_ref, &to_ref)
            .await
    } else {
        // Treat as branch comparison
        ui::create_tui_spinner(
            format!(
                "{} - Comparing branches: {} -> {}",
                random_message.text, from_ref, to_ref
            )
            .as_str(),
        )
        .tick();

        service
            .generate_pr_for_branch_diff(effective_instructions, &from_ref, &to_ref)
            .await
    }
}

/// Handle case where only --to parameter is provided
async fn handle_to_only_parameter(
    service: Arc<CommitService>,
    effective_instructions: &str,
    to_ref: String,
    random_message: &messages::ColoredMessage,
) -> Result<super::types::GeneratedPullRequest> {
    // Check if this is a single commit hash
    if is_likely_commit_hash(&to_ref) {
        // For a single commit specified with --to, compare it against its parent
        ui::create_tui_spinner(
            format!(
                "{} - Analyzing single commit: {}",
                random_message.text, to_ref
            )
            .as_str(),
        )
        .tick();

        service
            .generate_pr_for_commit_range(effective_instructions, &format!("{to_ref}^"), &to_ref)
            .await
    } else if is_commitish_syntax(&to_ref) {
        // For commitish like HEAD~2, compare it against its parent (single commit analysis)
        SpinnerState::with_message(
            format!(
                "{} - Analyzing single commit: {}",
                random_message.text, to_ref
            )
            .as_str(),
        );

        service
            .generate_pr_for_commit_range(effective_instructions, &format!("{to_ref}^"), &to_ref)
            .await
    } else {
        // Default from to "main" if only to is specified with a branch name
        SpinnerState::with_message(
            format!("{} - Comparing main -> {}", random_message.text, to_ref).as_str(),
        );

        service
            .generate_pr_for_branch_diff(effective_instructions, "main", &to_ref)
            .await
    }
}

/// Handle case where only --from parameter is provided
async fn handle_from_only_parameter(
    service: Arc<CommitService>,
    effective_instructions: &str,
    from_ref: String,
    random_message: &messages::ColoredMessage,
) -> Result<super::types::GeneratedPullRequest> {
    // Check if this looks like a single commit hash that we should compare against its parent
    if is_likely_commit_hash(&from_ref) {
        // For a single commit hash, compare it against its parent (commit^..commit)
        ui::create_tui_spinner(
            format!(
                "{} - Analyzing single commit: {}",
                random_message.text, from_ref
            )
            .as_str(),
        )
        .tick();

        service
            .generate_pr_for_commit_range(
                effective_instructions,
                &format!("{from_ref}^"),
                &from_ref,
            )
            .await
    } else if is_commitish_syntax(&from_ref) {
        // For commitish like HEAD~2, compare from that point to HEAD (reviewing multiple commits)
        ui::create_tui_spinner(
            format!(
                "{} - Analyzing range: {}..HEAD",
                random_message.text, from_ref
            )
            .as_str(),
        )
        .tick();

        service
            .generate_pr_for_commit_range(effective_instructions, &from_ref, "HEAD")
            .await
    } else {
        // For a branch name, compare to HEAD
        ui::create_tui_spinner(
            format!(
                "{} - Analyzing range: {}..HEAD",
                random_message.text, from_ref
            )
            .as_str(),
        )
        .tick();

        service
            .generate_pr_for_commit_range(effective_instructions, &from_ref, "HEAD")
            .await
    }
}

/// Handle case where no parameters are provided
async fn handle_no_parameters(
    service: Arc<CommitService>,
    effective_instructions: &str,
    random_message: &messages::ColoredMessage,
) -> Result<super::types::GeneratedPullRequest> {
    // This case should be caught by validation, but provide a sensible fallback
    ui::create_tui_spinner(format!("{} - Comparing main -> HEAD", random_message.text).as_str())
        .tick();

    service
        .generate_pr_for_branch_diff(effective_instructions, "main", "HEAD")
        .await
}

/// Heuristic to determine if a reference looks like a commit hash or commitish
fn is_likely_commit_hash_or_commitish(reference: &str) -> bool {
    // Check for commit hash (7+ hex chars)
    if reference.len() >= 7 && reference.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }

    // Check for Git commitish syntax
    is_commitish_syntax(reference)
}

/// Check if a reference uses Git commitish syntax
fn is_commitish_syntax(reference: &str) -> bool {
    // Common commitish patterns:
    // HEAD~2, HEAD^, @~3, main~1, origin/main^, etc.
    reference.contains('~') || reference.contains('^') || reference.starts_with('@')
}

/// Heuristic to determine if a reference looks like a commit hash (legacy function for backward compatibility)
fn is_likely_commit_hash(reference: &str) -> bool {
    reference.len() >= 7 && reference.chars().all(|c| c.is_ascii_hexdigit())
}
