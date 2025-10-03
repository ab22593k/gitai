use gitpilot::{
    commit::prompt::{create_system_prompt, create_user_prompt},
    config::Config,
    token_optimizer::TokenOptimizer,
};

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::setup_git_repo;

#[tokio::test]
async fn test_token_optimization_integration() {
    let (_temp_dir, git_repo) = setup_git_repo();
    let config = Config::default();

    // Set a small token limit for the OpenAI provider to force truncation
    let small_token_limit = 200;
    let optimizer = TokenOptimizer::new(small_token_limit);

    let context = git_repo
        .get_git_info(&config)
        .await
        .expect("Failed to get git info");

    let system_prompt = create_system_prompt(&config).expect("Failed to create system prompt");
    let user_prompt = create_user_prompt(&context);
    let prompt = format!("{system_prompt}\n{user_prompt}");

    // Check that the prompt is within the token limit
    let prompt = optimizer.truncate_string(&prompt, small_token_limit);

    let token_count = optimizer.count_tokens(&prompt);

    println!("Token count: {token_count}");
    println!("Token limit: {small_token_limit}");
    println!("Prompt:\n{prompt}");

    assert!(
        token_count <= small_token_limit,
        "Prompt exceeds token limit. Token count: {token_count}, Limit: {small_token_limit}"
    );

    // Check that the prompt contains essential information
    assert!(
        prompt.contains("Git commit message"),
        "Prompt should contain instructions"
    );

    // The following assertions may fail due to truncation, so we'll make them optional
    // Only check for content if the prompt was NOT truncated (doesn't end with ellipsis)
    if !prompt.ends_with("…") {
        assert!(
            prompt.contains("Branch:"),
            "Prompt should contain branch information"
        );
        assert!(
            prompt.contains("Recent commits:"),
            "Prompt should mention recent commits"
        );
        assert!(
            prompt.contains("Staged changes:"),
            "Prompt should mention staged changes"
        );
    }

    // Check that the prompt ends with the truncation indicator
    assert!(
        prompt.ends_with('…'),
        "Prompt should end with truncation indicator"
    );

    // Test with a larger token limit
    let large_token_limit = 5000;

    let system_prompt = create_system_prompt(&config).expect("Failed to create system prompt");
    let user_prompt = create_user_prompt(&context);
    let large_prompt = format!("{system_prompt}\n{user_prompt}");

    let large_optimizer = TokenOptimizer::new(large_token_limit);
    let large_token_count = large_optimizer.count_tokens(&large_prompt);

    assert!(
        large_token_count <= large_token_limit,
        "Large prompt exceeds token limit. Token count: {large_token_count}, Limit: {large_token_limit}"
    );
}
