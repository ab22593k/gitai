use super::service::CommitService;
use super::types::format_pull_request;
use crate::commands::common::run_with_spinner;
use crate::commands::common::service::create_commit_service;
use crate::common::CommonParams;
use crate::config::Config;
use crate::llm::messages;
use crate::output;
use crate::tui::spinner::SpinnerState;

use anyhow::Result;
use std::sync::Arc;

/// Handles the PR description generation command
pub async fn handle_pr_command(
    common: CommonParams,
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
    let spinner = output::create_tui_spinner(
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
        output::create_tui_spinner(
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
        output::create_tui_spinner(
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
        output::create_tui_spinner(
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
        output::create_tui_spinner(
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
        output::create_tui_spinner(
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
        output::create_tui_spinner(
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
        output::create_tui_spinner(
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
    output::create_tui_spinner(
        format!("{} - Comparing main -> HEAD", random_message.text).as_str(),
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_likely_commit_hash_valid() {
        assert!(is_likely_commit_hash("abc1234"));
        assert!(is_likely_commit_hash("deadbeef"));
        assert!(is_likely_commit_hash("ABC1234"));
        assert!(is_likely_commit_hash("a1b2c3d4e5f6"));
    }

    #[test]
    fn test_is_likely_commit_hash_too_short() {
        assert!(!is_likely_commit_hash("abc123"));
        assert!(!is_likely_commit_hash("a1b2c3"));
        assert!(!is_likely_commit_hash("abcdef"));
    }

    #[test]
    fn test_is_likely_commit_hash_non_hex() {
        assert!(!is_likely_commit_hash("abcdefg"));
        assert!(!is_likely_commit_hash("1234567z"));
        assert!(!is_likely_commit_hash("feature-branch"));
    }

    #[test]
    fn test_is_commitish_syntax_tilde() {
        assert!(is_commitish_syntax("HEAD~2"));
        assert!(is_commitish_syntax("main~1"));
        assert!(is_commitish_syntax("@~3"));
    }

    #[test]
    fn test_is_commitish_syntax_caret() {
        assert!(is_commitish_syntax("HEAD^"));
        assert!(is_commitish_syntax("origin/main^"));
        assert!(is_commitish_syntax("v1.0^"));
    }

    #[test]
    fn test_is_commitish_syntax_at_sign() {
        assert!(is_commitish_syntax("@"));
        assert!(is_commitish_syntax("@~1"));
        assert!(is_commitish_syntax("@{1}"));
    }

    #[test]
    fn test_is_commitish_syntax_plain_branch() {
        assert!(!is_commitish_syntax("main"));
        assert!(!is_commitish_syntax("feature/add-login"));
        assert!(!is_commitish_syntax("release-v2"));
    }

    #[test]
    fn test_is_likely_commit_hash_or_commitish_combined() {
        assert!(is_likely_commit_hash_or_commitish("abc1234"));
        assert!(is_likely_commit_hash_or_commitish("HEAD~2"));
        assert!(is_likely_commit_hash_or_commitish("@"));
        assert!(!is_likely_commit_hash_or_commitish("main"));
        assert!(!is_likely_commit_hash_or_commitish("feature/login"));
        assert!(!is_likely_commit_hash_or_commitish("abc12"));
    }

    #[test]
    fn test_is_likely_commit_hash_hex_edge_boundary() {
        assert!(is_likely_commit_hash("1234567"));
        assert!(!is_likely_commit_hash("123456"));
    }
}
