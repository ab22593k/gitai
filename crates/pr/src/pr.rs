use crate::models::GeneratedPullRequest;
use crate::prompt;
use anyhow::Result;
use claw_core::config::Config;
use claw_core::git::GitRepo;
use claw_core::llm::engine;
use claw_core::llm::messages;
use claw_core::output;
use claw_core::tui::spinner::SpinnerState;
use std::sync::Arc;

pub struct PullRequestStrategy {
    pub commit_messages: Vec<String>,
}

impl PullRequestStrategy {
    pub fn new(commit_messages: Vec<String>) -> Self {
        Self { commit_messages }
    }

    pub fn create_system_prompt(&self, config: &Config) -> Result<String> {
        prompt::create_pr_system_prompt(config)
    }

    pub fn create_user_prompt(&self, context: &claw_core::llm::context::CommitContext) -> String {
        prompt::create_pr_user_prompt(context, &self.commit_messages)
    }
}

async fn generate_pr(
    strategy: PullRequestStrategy,
    instructions: &str,
    context: Option<claw_core::llm::context::CommitContext>,
    config: &Config,
    provider_name: &str,
) -> Result<GeneratedPullRequest> {
    let mut config_clone = config.clone();
    config_clone.instructions = instructions.to_string();

    let Some(context) = context else {
        return Err(anyhow::anyhow!(
            "Commit context is required for PR generation"
        ));
    };

    let system_prompt = strategy.create_system_prompt(&config_clone)?;
    let user_prompt = strategy.create_user_prompt(&context);

    engine::get_message::<GeneratedPullRequest>(
        &config_clone,
        provider_name,
        &system_prompt,
        &user_prompt,
    )
    .await
}

pub async fn generate_pr_based_on_parameters(
    git_repo: Arc<GitRepo>,
    effective_instructions: &str,
    config: &Config,
    provider_name: &str,
    from: Option<String>,
    to: Option<String>,
) -> Result<GeneratedPullRequest> {
    let random_message = messages::get_waiting_message();
    let _spinner = output::SpinnerState::with_message(
        format!("{} - Generating PR description", random_message.text).as_str(),
    );

    match (from, to) {
        (Some(from_ref), Some(to_ref)) => {
            handle_from_and_to_parameters(
                git_repo,
                effective_instructions,
                config,
                provider_name,
                from_ref,
                to_ref,
                random_message,
            )
            .await
        }
        (None, Some(to_ref)) => {
            handle_to_only_parameter(
                git_repo,
                effective_instructions,
                config,
                provider_name,
                to_ref,
                random_message,
            )
            .await
        }
        (Some(from_ref), None) => {
            handle_from_only_parameter(
                git_repo,
                effective_instructions,
                config,
                provider_name,
                from_ref,
                random_message,
            )
            .await
        }
        (None, None) => {
            handle_no_parameters(
                git_repo,
                effective_instructions,
                config,
                provider_name,
                random_message,
            )
            .await
        }
    }
}

async fn handle_from_and_to_parameters(
    git_repo: Arc<GitRepo>,
    effective_instructions: &str,
    config: &Config,
    provider_name: &str,
    from_ref: String,
    to_ref: String,
    random_message: &messages::ColoredMessage,
) -> Result<GeneratedPullRequest> {
    if from_ref == to_ref {
        output::create_tui_spinner(
            format!(
                "{} - Analyzing single commit: {}",
                random_message.text, from_ref
            )
            .as_str(),
        )
        .tick();

        let context =
            git_repo.get_git_info_for_commit_range(config, &format!("{from_ref}^"), &from_ref)?;
        let commit_messages = git_repo.get_commits_for_pr(&format!("{from_ref}^"), &from_ref)?;
        let strategy = PullRequestStrategy::new(commit_messages);
        generate_pr(
            strategy,
            effective_instructions,
            Some(context),
            config,
            provider_name,
        )
        .await
    } else if is_likely_commit_hash_or_commitish(&from_ref)
        || is_likely_commit_hash_or_commitish(&to_ref)
    {
        output::create_tui_spinner(
            format!(
                "{} - Analyzing commit range: {}..{}",
                random_message.text, from_ref, to_ref
            )
            .as_str(),
        )
        .tick();

        let context = git_repo.get_git_info_for_commit_range(config, &from_ref, &to_ref)?;
        let commit_messages = git_repo.get_commits_for_pr(&from_ref, &to_ref)?;
        let strategy = PullRequestStrategy::new(commit_messages);
        generate_pr(
            strategy,
            effective_instructions,
            Some(context),
            config,
            provider_name,
        )
        .await
    } else {
        output::create_tui_spinner(
            format!(
                "{} - Comparing branches: {} -> {}",
                random_message.text, from_ref, to_ref
            )
            .as_str(),
        )
        .tick();

        let context = git_repo.get_git_info_for_branch_diff(config, &from_ref, &to_ref)?;
        let commit_messages = git_repo.get_commits_for_pr(&from_ref, &to_ref)?;
        let strategy = PullRequestStrategy::new(commit_messages);
        generate_pr(
            strategy,
            effective_instructions,
            Some(context),
            config,
            provider_name,
        )
        .await
    }
}

async fn handle_to_only_parameter(
    git_repo: Arc<GitRepo>,
    effective_instructions: &str,
    config: &Config,
    provider_name: &str,
    to_ref: String,
    random_message: &messages::ColoredMessage,
) -> Result<GeneratedPullRequest> {
    if is_likely_commit_hash(&to_ref) {
        output::create_tui_spinner(
            format!(
                "{} - Analyzing single commit: {}",
                random_message.text, to_ref
            )
            .as_str(),
        )
        .tick();

        let context =
            git_repo.get_git_info_for_commit_range(config, &format!("{to_ref}^"), &to_ref)?;
        let commit_messages = git_repo.get_commits_for_pr(&format!("{to_ref}^"), &to_ref)?;
        let strategy = PullRequestStrategy::new(commit_messages);
        generate_pr(
            strategy,
            effective_instructions,
            Some(context),
            config,
            provider_name,
        )
        .await
    } else if is_commitish_syntax(&to_ref) {
        SpinnerState::with_message(
            format!(
                "{} - Analyzing single commit: {}",
                random_message.text, to_ref
            )
            .as_str(),
        );

        let context =
            git_repo.get_git_info_for_commit_range(config, &format!("{to_ref}^"), &to_ref)?;
        let commit_messages = git_repo.get_commits_for_pr(&format!("{to_ref}^"), &to_ref)?;
        let strategy = PullRequestStrategy::new(commit_messages);
        generate_pr(
            strategy,
            effective_instructions,
            Some(context),
            config,
            provider_name,
        )
        .await
    } else {
        SpinnerState::with_message(
            format!("{} - Comparing main -> {}", random_message.text, to_ref).as_str(),
        );

        let context = git_repo.get_git_info_for_branch_diff(config, "main", &to_ref)?;
        let commit_messages = git_repo.get_commits_for_pr("main", &to_ref)?;
        let strategy = PullRequestStrategy::new(commit_messages);
        generate_pr(
            strategy,
            effective_instructions,
            Some(context),
            config,
            provider_name,
        )
        .await
    }
}

async fn handle_from_only_parameter(
    git_repo: Arc<GitRepo>,
    effective_instructions: &str,
    config: &Config,
    provider_name: &str,
    from_ref: String,
    random_message: &messages::ColoredMessage,
) -> Result<GeneratedPullRequest> {
    if is_likely_commit_hash(&from_ref) {
        output::create_tui_spinner(
            format!(
                "{} - Analyzing single commit: {}",
                random_message.text, from_ref
            )
            .as_str(),
        )
        .tick();

        let context =
            git_repo.get_git_info_for_commit_range(config, &format!("{from_ref}^"), &from_ref)?;
        let commit_messages = git_repo.get_commits_for_pr(&format!("{from_ref}^"), &from_ref)?;
        let strategy = PullRequestStrategy::new(commit_messages);
        generate_pr(
            strategy,
            effective_instructions,
            Some(context),
            config,
            provider_name,
        )
        .await
    } else {
        output::create_tui_spinner(
            format!(
                "{} - Analyzing range: {}..HEAD",
                random_message.text, from_ref
            )
            .as_str(),
        )
        .tick();

        let context = git_repo.get_git_info_for_commit_range(config, &from_ref, "HEAD")?;
        let commit_messages = git_repo.get_commits_for_pr(&from_ref, "HEAD")?;
        let strategy = PullRequestStrategy::new(commit_messages);
        generate_pr(
            strategy,
            effective_instructions,
            Some(context),
            config,
            provider_name,
        )
        .await
    }
}

async fn handle_no_parameters(
    git_repo: Arc<GitRepo>,
    effective_instructions: &str,
    config: &Config,
    provider_name: &str,
    random_message: &messages::ColoredMessage,
) -> Result<GeneratedPullRequest> {
    output::create_tui_spinner(
        format!("{} - Comparing main -> HEAD", random_message.text).as_str(),
    )
    .tick();

    let context = git_repo.get_git_info_for_branch_diff(config, "main", "HEAD")?;
    let commit_messages = git_repo.get_commits_for_pr("main", "HEAD")?;
    let strategy = PullRequestStrategy::new(commit_messages);
    generate_pr(
        strategy,
        effective_instructions,
        Some(context),
        config,
        provider_name,
    )
    .await
}

fn is_likely_commit_hash_or_commitish(reference: &str) -> bool {
    if reference.len() >= 7 && reference.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }
    is_commitish_syntax(reference)
}

fn is_commitish_syntax(reference: &str) -> bool {
    reference.contains('~') || reference.contains('^') || reference.starts_with('@')
}

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
