use crate::{config::Config, core::context::CommitContext};
use log::debug;
use tiktoken_rs::cl100k_base;

pub struct TokenOptimizer {
    encoder: tiktoken_rs::CoreBPE,
    max_tokens: usize,
    #[allow(dead_code)]
    config: Config,
}

#[derive(Debug)]
pub enum TokenError {
    EncoderInit(String),
    EncodingFailed(String),
    DecodingFailed(String),
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenError::EncoderInit(e) => write!(f, "Failed to initialize encoder: {e}"),
            TokenError::EncodingFailed(e) => write!(f, "Encoding failed: {e}"),
            TokenError::DecodingFailed(e) => write!(f, "Decoding failed: {e}"),
        }
    }
}

impl std::error::Error for TokenError {}

#[derive(Debug)]
struct ContextItem {
    item_type: ContextItemType,
    token_count: usize,
    importance: f32,
}

#[derive(Debug)]
enum ContextItemType {
    Diff { file_index: usize },
    Commit { commit_index: usize },
    Content { file_index: usize },
}

impl TokenOptimizer {
    pub fn new(max_tokens: usize, config: Config) -> Result<Self, TokenError> {
        let encoder = cl100k_base().map_err(|e| TokenError::EncoderInit(e.to_string()))?;

        Ok(Self {
            encoder,
            max_tokens,
            config,
        })
    }

    /// Create a token optimizer for counting only (no config needed)
    pub fn for_counting() -> Result<Self, TokenError> {
        let encoder = cl100k_base().map_err(|e| TokenError::EncoderInit(e.to_string()))?;

        Ok(Self {
            encoder,
            max_tokens: 0,             // Not used for counting
            config: Config::default(), // Not used for counting
        })
    }

    #[allow(clippy::unused_async)]
    pub async fn optimize_context(&self, context: &mut CommitContext) -> Result<(), TokenError> {
        let context_items = self.calculate_context_items(context);
        self.allocate_tokens_proportionally(context, context_items);
        Ok(())
    }

    // Define base importance multipliers for different context types
    // Staged changes (diffs) are most important, then recent commits, then file contents
    const DIFF_BASE_MULTIPLIER: f32 = 3.0; // Highest priority - current changes
    const COMMIT_BASE_MULTIPLIER: f32 = 2.0; // Medium priority - recent history
    const CONTENT_BASE_MULTIPLIER: f32 = 1.0; // Lower priority - supporting context

    fn calculate_context_items(&self, context: &CommitContext) -> Vec<ContextItem> {
        let mut context_items = Vec::new();

        // Add diffs with importance scores (staged changes are most important)
        for (i, file) in context.staged_files.iter().enumerate() {
            let token_count = self.count_tokens(&file.diff);
            // Importance = base_multiplier * token_count * change_type_factor
            let change_type_factor = match file.change_type {
                crate::core::context::ChangeType::Added => 1.2, // New files are important
                crate::core::context::ChangeType::Modified => 1.0, // Standard modifications
                crate::core::context::ChangeType::Deleted => 0.8, // Deletions less important
            };
            #[allow(clippy::cast_precision_loss, clippy::as_conversions)]
            let importance = Self::DIFF_BASE_MULTIPLIER * token_count as f32 * change_type_factor;
            context_items.push(ContextItem {
                item_type: ContextItemType::Diff { file_index: i },
                token_count,
                importance,
            });
        }

        // Add commits with importance scores (recent commits are important for context)
        for (i, commit) in context.recent_commits.iter().enumerate() {
            let token_count = self.count_tokens(&commit.message);
            // Importance = base_multiplier * token_count * recency_factor * length_factor
            #[allow(clippy::cast_precision_loss, clippy::as_conversions)]
            let recency_factor = 1.0 / (i + 1) as f32; // Earlier commits more important
            let length_factor = if token_count > 50 { 1.2 } else { 1.0 }; // Longer messages may be more informative
            #[allow(clippy::cast_precision_loss, clippy::as_conversions)]
            let importance =
                Self::COMMIT_BASE_MULTIPLIER * token_count as f32 * recency_factor * length_factor;
            context_items.push(ContextItem {
                item_type: ContextItemType::Commit { commit_index: i },
                token_count,
                importance,
            });
        }

        // Add file contents with importance scores (supporting context, lowest priority)
        for (i, file) in context.staged_files.iter().enumerate() {
            if let Some(content) = &file.content {
                let token_count = self.count_tokens(content);
                // Importance = base_multiplier * token_count * relevance_factor * size_factor
                let relevance_factor = 1.0; // All staged files are equally relevant
                let size_factor = if token_count > 100 { 0.8 } else { 1.0 }; // Very large files get slightly lower priority
                #[allow(clippy::cast_precision_loss, clippy::as_conversions)]
                let importance = Self::CONTENT_BASE_MULTIPLIER
                    * token_count as f32
                    * relevance_factor
                    * size_factor;
                context_items.push(ContextItem {
                    item_type: ContextItemType::Content { file_index: i },
                    token_count,
                    importance,
                });
            }
        }

        context_items
    }

    fn allocate_tokens_proportionally(
        &self,
        context: &mut CommitContext,
        mut context_items: Vec<ContextItem>,
    ) {
        // Sort by importance (highest first)
        context_items.sort_by(|a, b| {
            b.importance
                .partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Allocate tokens proportionally based on importance
        let total_importance: f32 = context_items.iter().map(|item| item.importance).sum();
        let mut remaining_tokens = self.max_tokens;

        for item in &context_items {
            if remaining_tokens == 0 {
                break;
            }

            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss,
                clippy::as_conversions
            )]
            let allocated_tokens = if total_importance > 0.0 {
                ((item.importance / total_importance) * self.max_tokens as f32) as usize
            } else {
                0
            }
            .min(item.token_count)
            .min(remaining_tokens);

            if allocated_tokens < item.token_count {
                // Need to truncate this item
                match &item.item_type {
                    ContextItemType::Diff { file_index } => {
                        if let Some(file) = context.staged_files.get_mut(*file_index) {
                            debug!(
                                "Truncating diff for {path} from {original} to {allocated} tokens",
                                path = file.path,
                                original = item.token_count,
                                allocated = allocated_tokens
                            );
                            let _ = self
                                .truncate_string(&file.diff, allocated_tokens)
                                .map(|truncated| file.diff = truncated);
                        }
                    }
                    ContextItemType::Commit { commit_index } => {
                        if let Some(commit) = context.recent_commits.get_mut(*commit_index) {
                            debug!(
                                "Truncating commit message from {original} to {allocated} tokens",
                                original = item.token_count,
                                allocated = allocated_tokens
                            );
                            let _ = self
                                .truncate_string(&commit.message, allocated_tokens)
                                .map(|truncated| commit.message = truncated);
                        }
                    }
                    ContextItemType::Content { file_index } => {
                        if let Some(file) = context.staged_files.get_mut(*file_index) {
                            if let Some(content) = &mut file.content {
                                debug!(
                                    "Truncating content for {path} from {original} to {allocated} tokens",
                                    path = file.path,
                                    original = item.token_count,
                                    allocated = allocated_tokens
                                );
                                let _ = self
                                    .truncate_string(content, allocated_tokens)
                                    .map(|truncated| *content = truncated);
                            }
                        }
                    }
                }
            }

            remaining_tokens = remaining_tokens.saturating_sub(allocated_tokens);
        }

        // Clear any remaining items that didn't get tokens
        if remaining_tokens == 0 {
            // Clear remaining low-importance items
            #[allow(clippy::match_same_arms)]
            for item in context_items
                .iter()
                .skip_while(|item| match &item.item_type {
                    ContextItemType::Diff { .. } => true,
                    ContextItemType::Commit { .. } => true,
                    ContextItemType::Content { .. } => false,
                })
            {
                if let ContextItemType::Content { file_index } = &item.item_type {
                    if let Some(file) = context.staged_files.get_mut(*file_index) {
                        file.content = None;
                        file.content_excluded = true;
                    }
                }
            }
        }

        debug!(
            "Optimized context with importance weighting, final token usage: {}",
            self.max_tokens - remaining_tokens
        );
    }

    pub fn truncate_string(&self, s: &str, max_tokens: usize) -> Result<String, TokenError> {
        let tokens = self.encoder.encode_ordinary(s);

        if tokens.len() <= max_tokens {
            return Ok(s.to_string());
        }

        if max_tokens == 0 {
            return Ok(String::from("…"));
        }

        // Try to find a good truncation point that avoids mid-sentence cuts
        let truncation_point = self.find_sentence_boundary(s, max_tokens);

        if truncation_point == 0 {
            // No good sentence boundary found, fall back to token-based truncation
            let truncation_limit = max_tokens.saturating_sub(1);
            let ellipsis_token = self
                .encoder
                .encode_ordinary("…")
                .first()
                .copied()
                .ok_or_else(|| {
                    TokenError::EncodingFailed("Failed to encode ellipsis".to_string())
                })?;

            let mut truncated_tokens = Vec::with_capacity(truncation_limit + 1);
            truncated_tokens.extend_from_slice(&tokens[..truncation_limit]);
            truncated_tokens.push(ellipsis_token);

            return self
                .encoder
                .decode(truncated_tokens)
                .map_err(|e| TokenError::DecodingFailed(e.to_string()));
        }

        // Truncate at the sentence boundary
        let truncated_text = &s[..truncation_point];
        let truncated_with_ellipsis = format!("{}…", truncated_text.trim_end());

        // Check if this fits within token limit
        let final_tokens = self.encoder.encode_ordinary(&truncated_with_ellipsis);
        if final_tokens.len() <= max_tokens {
            Ok(truncated_with_ellipsis)
        } else {
            // If it doesn't fit, fall back to token-based truncation
            let truncation_limit = max_tokens.saturating_sub(1);
            let ellipsis_token = self
                .encoder
                .encode_ordinary("…")
                .first()
                .copied()
                .ok_or_else(|| {
                    TokenError::EncodingFailed("Failed to encode ellipsis".to_string())
                })?;

            let mut truncated_tokens = Vec::with_capacity(truncation_limit + 1);
            truncated_tokens.extend_from_slice(&tokens[..truncation_limit]);
            truncated_tokens.push(ellipsis_token);

            self.encoder
                .decode(truncated_tokens)
                .map_err(|e| TokenError::DecodingFailed(e.to_string()))
        }
    }

    /// Find a good sentence boundary within the token limit
    #[allow(clippy::unnecessary_wraps)]
    fn find_sentence_boundary(&self, s: &str, max_tokens: usize) -> usize {
        // Look for sentence endings: ., !, ?
        let sentence_endings = ['.', '!', '?'];

        // Start from a position that would give us roughly the right number of tokens
        let chars: Vec<char> = s.chars().collect();
        let mut best_boundary = 0;

        // Try to find sentence boundaries working backwards from the token limit
        for (i, &ch) in chars.iter().enumerate().rev() {
            if sentence_endings.contains(&ch) {
                // Check if this position would fit within our token limit
                let candidate_text = &s[..=i];
                let candidate_tokens = self.encoder.encode_ordinary(candidate_text);

                if candidate_tokens.len() <= max_tokens.saturating_sub(1) {
                    // Reserve space for ellipsis
                    // Check if this is followed by whitespace or end of string
                    let next_char = chars.get(i + 1);
                    if next_char.is_none() || next_char.is_some_and(|c| c.is_whitespace()) {
                        best_boundary = i + 1; // Include the sentence ending
                        break;
                    }
                }
            }

            // If we've gone too far back, stop
            if i < s.len() / 4 {
                break;
            }
        }

        best_boundary
    }

    #[inline]
    pub fn count_tokens(&self, s: &str) -> usize {
        self.encoder.encode_ordinary(s).len()
    }

    /// Summarize text using LLM
    #[allow(dead_code)]
    async fn summarize_text(&self, text: &str, max_tokens: usize) -> Result<String, TokenError> {
        let system_prompt = "You are a code diff summarizer. Provide a concise summary of the changes in the given diff, focusing on what was added, modified, or removed.";
        let user_prompt =
            format!("Summarize the following diff in {max_tokens} tokens or less:\n\n{text}");

        match crate::core::llm::get_message::<String>(
            &self.config,
            &self.config.default_provider,
            system_prompt,
            &user_prompt,
        )
        .await
        {
            Ok(summary) => Ok(summary),
            Err(e) => Err(TokenError::EncodingFailed(format!(
                "Summarization failed: {e}"
            ))),
        }
    }

    /// Perform hierarchical summarization (map-reduce) on large text
    #[allow(dead_code)]
    async fn hierarchical_summarize(
        &self,
        text: &str,
        max_tokens: usize,
    ) -> Result<String, TokenError> {
        // Try to summarize, but fall back to truncation if LLM fails
        if let Ok(summary) = self.try_hierarchical_summarize(text, max_tokens).await {
            Ok(summary)
        } else {
            // Fallback to truncation
            debug!("Summarization failed, falling back to truncation");
            self.truncate_string(text, max_tokens)
        }
    }

    #[allow(dead_code)]
    async fn try_hierarchical_summarize(
        &self,
        text: &str,
        max_tokens: usize,
    ) -> Result<String, TokenError> {
        // Split text into chunks that fit within LLM context
        let chunk_size = 4000; // Conservative chunk size for LLM input
        let chunks: Vec<&str> = text
            .as_bytes()
            .chunks(chunk_size)
            .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
            .filter(|chunk| !chunk.is_empty())
            .collect();

        if chunks.len() <= 1 {
            // If only one chunk, summarize directly
            return self.summarize_text(text, max_tokens).await;
        }

        // Map: Summarize each chunk
        let mut chunk_summaries = Vec::new();
        for chunk in &chunks {
            let summary = self
                .summarize_text(chunk, max_tokens / chunks.len())
                .await?;
            chunk_summaries.push(summary);
        }

        // Reduce: Combine summaries
        let combined = chunk_summaries.join("\n\n");
        if self.count_tokens(&combined) <= max_tokens {
            Ok(combined)
        } else {
            // If still too large, summarize the combined summaries
            self.summarize_text(&combined, max_tokens).await
        }
    }
}
