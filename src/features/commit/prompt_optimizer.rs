use crate::config::Config;
use crate::core::context::CommitContext;
use crate::core::model_info::ModelInfoService;
use crate::core::token_optimizer::TokenOptimizer;
use log::debug;

/// Optimize prompt to fit within token limits
///
/// This is a shared helper function that handles token counting, context optimization,
/// and prompt truncation to ensure the final prompt fits within the provider's token limit.
///
/// Token limit resolution priority:
/// 1. User-configured `token_limit` in provider config
/// 2. Dynamic fetch from provider API (Google, Groq, `OpenRouter`)
/// 3. Hardcoded fallback based on provider/model
///
/// # Arguments
///
/// * `config` - Configuration with preset and instructions
/// * `provider_name` - The name of the LLM provider
/// * `system_prompt` - The system prompt to use
/// * `context` - The commit context to optimize
/// * `create_user_prompt_fn` - A function that creates a user prompt from a context
///
/// # Returns
///
/// A tuple containing the optimized context and final user prompt
pub async fn optimize_prompt<F>(
    config: &Config,
    provider_name: &str,
    system_prompt: &str,
    mut context: CommitContext,
    create_user_prompt_fn: F,
) -> (CommitContext, String)
where
    F: Fn(&CommitContext) -> String,
{
    // Get model name and API key from config
    let (model_name, api_key) = config
        .providers
        .get(provider_name)
        .map_or(("", ""), |p| (p.model_name.as_str(), p.api_key.as_str()));

    // Determine token limit with priority:
    // 1. User-configured limit takes precedence
    // 2. Otherwise fetch from API (with cache) or use fallback
    let token_limit = if let Some(limit) = config
        .providers
        .get(provider_name)
        .and_then(|p| p.token_limit)
    {
        debug!("Using user-configured token limit: {limit}");
        limit
    } else {
        let limit = ModelInfoService::global()
            .get_token_limit(provider_name, model_name, api_key)
            .await;
        debug!("Token limit for {provider_name}/{model_name}: {limit}");
        limit
    };

    // Create a token optimizer to count tokens
    let optimizer = TokenOptimizer::for_counting().expect("Failed to create TokenOptimizer");
    let system_tokens = optimizer.count_tokens(system_prompt);

    debug!("Token limit: {token_limit}");
    debug!("System prompt tokens: {system_tokens}");

    // Reserve tokens for system prompt and some buffer for formatting
    // 1000 token buffer provides headroom for model responses and formatting
    let context_token_limit = token_limit.saturating_sub(system_tokens + 1000);
    debug!("Available tokens for context: {context_token_limit}");

    // Count tokens before optimization
    let user_prompt_before = create_user_prompt_fn(&context);
    let total_tokens_before = system_tokens + optimizer.count_tokens(&user_prompt_before);
    debug!("Total tokens before optimization: {total_tokens_before}");

    // Optimize the context with remaining token budget
    context.optimize(context_token_limit, config).await;

    let user_prompt = create_user_prompt_fn(&context);
    let user_tokens = optimizer.count_tokens(&user_prompt);
    let total_tokens = system_tokens + user_tokens;

    debug!("User prompt tokens after optimization: {user_tokens}");
    debug!("Total tokens after optimization: {total_tokens}");

    // If we're still over the limit, truncate the user prompt directly
    // 100 token safety buffer ensures we stay under the limit
    let final_user_prompt = if total_tokens > token_limit {
        debug!(
            "Total tokens {total_tokens} still exceeds limit {token_limit}, truncating user prompt"
        );
        let max_user_tokens = token_limit.saturating_sub(system_tokens + 100);
        optimizer
            .truncate_string(&user_prompt, max_user_tokens)
            .expect("Failed to truncate user prompt")
    } else {
        user_prompt
    };

    let final_tokens = system_tokens + optimizer.count_tokens(&final_user_prompt);
    debug!("Final total tokens after potential truncation: {final_tokens}");

    (context, final_user_prompt)
}
