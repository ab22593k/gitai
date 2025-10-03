use super::format_commit_result;
use super::service::CommitService;
use super::types::{format_commit_message, format_pull_request};
use crate::common::CommonParams;
use crate::config::Config;
use crate::git::GitRepo;
use crate::instruction_presets::PresetType;
use crate::messages;
use crate::tui::run_tui_commit;
use crate::ui;
use anyhow::{Context, Result};
use std::sync::Arc;

#[allow(clippy::fn_params_excessive_bools)]
#[allow(clippy::too_many_lines)]
pub async fn handle_message_command(
    common: CommonParams,
    auto_commit: bool,
    use_emoji: bool,
    print: bool,
    verify: bool,
    repository_url: Option<String>,
) -> Result<()> {
    // Check if the preset is appropriate for commit messages
    if !common.is_valid_preset_for_type(PresetType::Commit) {
        ui::print_warning(
            "The specified preset may not be suitable for commit messages. Consider using a commit or general preset instead.",
        );
        ui::print_info("Run 'gitpilot presets' to see available presets for commits.");
    }

    let mut config = Config::load()?;
    common.apply_to_config(&mut config)?;

    // Create the service using the common function
    let service = create_commit_service(
        &common,
        repository_url,
        &config,
        use_emoji && config.use_emoji,
        verify,
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

    if git_info.staged_files.is_empty() {
        ui::print_warning(
            "No staged changes. Please stage your changes before generating a commit message.",
        );
        ui::print_info("You can stage changes using 'git add <file>' or 'git add .'");
        return Ok(());
    }

    // Run pre-commit hook before we do anything else
    if let Err(e) = service.pre_commit() {
        ui::print_error(&format!("Pre-commit failed: {e}"));
        return Err(e);
    }

    let effective_instructions = common
        .instructions
        .unwrap_or_else(|| config.instructions.clone());
    let preset_str = common.preset.as_deref().unwrap_or("");

    // Create and start the spinner
    let spinner = ui::create_spinner("");
    let random_message = messages::get_waiting_message();
    spinner.set_message(random_message.text);

    // Generate an initial message
    let initial_message = service
        .generate_message(preset_str, &effective_instructions)
        .await?;

    // Stop the spinner
    spinner.finish_and_clear();

    if print {
        println!("{}", format_commit_message(&initial_message));
        return Ok(());
    }

    if auto_commit {
        // Only allow auto-commit for local repositories
        if service.is_remote_repository() {
            ui::print_error(
                "Cannot automatically commit to a remote repository. Use --print instead.",
            );
            return Err(anyhow::anyhow!(
                "Auto-commit not supported for remote repositories"
            ));
        }

        match service.perform_commit(&format_commit_message(&initial_message)) {
            Ok(result) => {
                let output =
                    format_commit_result(&result, &format_commit_message(&initial_message));
                println!("{output}");
            }
            Err(e) => {
                eprintln!("Failed to commit: {e}");
                return Err(e);
            }
        }
        return Ok(());
    }

    // Only allow interactive commit for local repositories
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
        String::from(preset_str),
        git_info.user_name,
        git_info.user_email,
        service,
        config.use_emoji,
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
    // Check if the preset is appropriate for PR descriptions
    if !common.is_valid_preset_for_type(PresetType::Review)
        && !common.is_valid_preset_for_type(PresetType::Both)
    {
        ui::print_warning(
            "The specified preset may not be suitable for PR descriptions. Consider using a review or general preset instead.",
        );
        ui::print_info("Run 'gitpilot presets' to see available presets for PRs.");
    }

    // Validate parameter combinations
    validate_pr_parameters(from.as_ref(), to.as_ref());

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

/// Validates the parameter combinations for PR command
fn validate_pr_parameters(_from: Option<&String>, _to: Option<&String>) {
    // Now that we provide sensible defaults, we only need to validate if the parameters make sense
    // All combinations are valid:
    // - from + to: explicit range/branch comparison
    // - from only: from..HEAD
    // - to only: main..to
    // - none: main..HEAD (caught earlier, but handled gracefully)

    // No validation errors needed - all combinations are handled
}

/// Sets up the PR service with proper configuration
fn setup_pr_service(
    common: &CommonParams,
    repository_url: Option<String>,
    config: &Config,
) -> Result<Arc<CommitService>> {
    // Use the common function for service creation
    create_commit_service(
        common,
        repository_url,
        config,
        config.use_emoji, // Use emoji setting from config for PR descriptions
        false,            // verification not needed for PR descriptions
    )
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
    let preset_str = common.preset.as_deref().unwrap_or("");

    // Create and start the spinner
    let spinner = ui::create_spinner("");
    let random_message = messages::get_waiting_message();
    spinner.set_message(format!(
        "{} - Generating PR description",
        random_message.text
    ));

    let pr_description = match (from, to) {
        (Some(from_ref), Some(to_ref)) => {
            handle_from_and_to_parameters(
                service,
                preset_str,
                &effective_instructions,
                from_ref,
                to_ref,
                &random_message,
            )
            .await?
        }
        (None, Some(to_ref)) => {
            handle_to_only_parameter(
                service,
                preset_str,
                &effective_instructions,
                to_ref,
                &random_message,
            )
            .await?
        }
        (Some(from_ref), None) => {
            handle_from_only_parameter(
                service,
                preset_str,
                &effective_instructions,
                from_ref,
                &random_message,
            )
            .await?
        }
        (None, None) => {
            handle_no_parameters(
                service,
                preset_str,
                &effective_instructions,
                &random_message,
            )
            .await?
        }
    };

    // Stop the spinner
    spinner.finish_and_clear();

    Ok(pr_description)
}

/// Handle case where both --from and --to parameters are provided
async fn handle_from_and_to_parameters(
    service: Arc<CommitService>,
    preset_str: &str,
    effective_instructions: &str,
    from_ref: String,
    to_ref: String,
    random_message: &crate::messages::ColoredMessage,
) -> Result<super::types::GeneratedPullRequest> {
    // Special case: if from and to are the same, treat as single commit analysis
    if from_ref == to_ref {
        let spinner = ui::create_spinner("");
        spinner.set_message(format!(
            "{} - Analyzing single commit: {}",
            random_message.text, from_ref
        ));

        service
            .generate_pr_for_commit_range(
                preset_str,
                effective_instructions,
                &format!("{from_ref}^"), // Parent of the commit
                &from_ref,               // The commit itself
            )
            .await
    } else if is_likely_commit_hash_or_commitish(&from_ref)
        || is_likely_commit_hash_or_commitish(&to_ref)
    {
        // Check if these look like commit hashes (7+ hex chars) or branches
        // Treat as commit range
        let spinner = ui::create_spinner("");
        spinner.set_message(format!(
            "{} - Analyzing commit range: {}..{}",
            random_message.text, from_ref, to_ref
        ));

        service
            .generate_pr_for_commit_range(preset_str, effective_instructions, &from_ref, &to_ref)
            .await
    } else {
        // Treat as branch comparison
        let spinner = ui::create_spinner("");
        spinner.set_message(format!(
            "{} - Comparing branches: {} -> {}",
            random_message.text, from_ref, to_ref
        ));

        service
            .generate_pr_for_branch_diff(preset_str, effective_instructions, &from_ref, &to_ref)
            .await
    }
}

/// Handle case where only --to parameter is provided
async fn handle_to_only_parameter(
    service: Arc<CommitService>,
    preset_str: &str,
    effective_instructions: &str,
    to_ref: String,
    random_message: &crate::messages::ColoredMessage,
) -> Result<super::types::GeneratedPullRequest> {
    let spinner = ui::create_spinner("");

    // Check if this is a single commit hash
    if is_likely_commit_hash(&to_ref) {
        // For a single commit specified with --to, compare it against its parent
        spinner.set_message(format!(
            "{} - Analyzing single commit: {}",
            random_message.text, to_ref
        ));

        service
            .generate_pr_for_commit_range(
                preset_str,
                effective_instructions,
                &format!("{to_ref}^"), // Parent of the commit
                &to_ref,               // The commit itself
            )
            .await
    } else if is_commitish_syntax(&to_ref) {
        // For commitish like HEAD~2, compare it against its parent (single commit analysis)
        spinner.set_message(format!(
            "{} - Analyzing single commit: {}",
            random_message.text, to_ref
        ));

        service
            .generate_pr_for_commit_range(
                preset_str,
                effective_instructions,
                &format!("{to_ref}^"), // Parent of the commitish
                &to_ref,               // The commitish itself
            )
            .await
    } else {
        // Default from to "main" if only to is specified with a branch name
        spinner.set_message(format!(
            "{} - Comparing main -> {}",
            random_message.text, to_ref
        ));

        service
            .generate_pr_for_branch_diff(preset_str, effective_instructions, "main", &to_ref)
            .await
    }
}

/// Handle case where only --from parameter is provided
async fn handle_from_only_parameter(
    service: Arc<CommitService>,
    preset_str: &str,
    effective_instructions: &str,
    from_ref: String,
    random_message: &crate::messages::ColoredMessage,
) -> Result<super::types::GeneratedPullRequest> {
    let spinner = ui::create_spinner("");

    // Check if this looks like a single commit hash that we should compare against its parent
    if is_likely_commit_hash(&from_ref) {
        // For a single commit hash, compare it against its parent (commit^..commit)
        spinner.set_message(format!(
            "{} - Analyzing single commit: {}",
            random_message.text, from_ref
        ));

        service
            .generate_pr_for_commit_range(
                preset_str,
                effective_instructions,
                &format!("{from_ref}^"), // Parent of the commit
                &from_ref,               // The commit itself
            )
            .await
    } else if is_commitish_syntax(&from_ref) {
        // For commitish like HEAD~2, compare from that point to HEAD (reviewing multiple commits)
        spinner.set_message(format!(
            "{} - Analyzing range: {}..HEAD",
            random_message.text, from_ref
        ));

        service
            .generate_pr_for_commit_range(preset_str, effective_instructions, &from_ref, "HEAD")
            .await
    } else {
        // For a branch name, compare to HEAD
        spinner.set_message(format!(
            "{} - Analyzing range: {}..HEAD",
            random_message.text, from_ref
        ));

        service
            .generate_pr_for_commit_range(preset_str, effective_instructions, &from_ref, "HEAD")
            .await
    }
}

/// Handle case where no parameters are provided
async fn handle_no_parameters(
    service: Arc<CommitService>,
    preset_str: &str,
    effective_instructions: &str,
    random_message: &crate::messages::ColoredMessage,
) -> Result<super::types::GeneratedPullRequest> {
    // This case should be caught by validation, but provide a sensible fallback
    let spinner = ui::create_spinner("");
    spinner.set_message(format!("{} - Comparing main -> HEAD", random_message.text));

    service
        .generate_pr_for_branch_diff(preset_str, effective_instructions, "main", "HEAD")
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

/// Common function to set up `CommitService`
fn create_commit_service(
    common: &CommonParams,
    repository_url: Option<String>,
    config: &Config,
    use_emoji: bool,
    verify: bool,
) -> Result<Arc<CommitService>> {
    // Combine repository URL from CLI and CommonParams
    let repo_url = repository_url.or(common.repository_url.clone());

    // Create the git repository
    let git_repo = GitRepo::new_from_url(repo_url).context("Failed to create GitRepo")?;

    let repo_path = git_repo.repo_path().clone();
    let provider_name = &config.default_provider;

    let service = Arc::new(
        CommitService::new(
            config.clone(),
            &repo_path,
            provider_name,
            use_emoji,
            verify,
            git_repo,
        )
        .context("Failed to create CommitService")?,
    );

    // Check environment prerequisites
    service
        .check_environment()
        .context("Environment check failed")?;

    Ok(service)
}
