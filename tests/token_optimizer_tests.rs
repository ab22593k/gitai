use gait::{
    Config,
    core::{
        context::{ChangeType, CommitContext, RecentCommit, StagedFile},
        token_optimizer::TokenOptimizer,
    },
};

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::MockDataBuilder;

const DEBUG: bool = false;

fn create_test_config() -> Config {
    Config::default()
}

// Helper function to create a test context with additional commits and files
fn create_test_context() -> CommitContext {
    let mut context = MockDataBuilder::commit_context();

    // Add second commit for testing
    context.recent_commits.push(RecentCommit {
        hash: "def456".to_string(),
        message: "Add new feature".to_string(),
        author: "Test Author".to_string(),
        timestamp: "2023-01-02 00:00:00".to_string(),
    });

    // Replace staged files with test-specific ones that have content
    context.staged_files = vec![
        StagedFile {
            path: "file1.rs".to_string(),
            change_type: ChangeType::Modified,
            diff: "- Old line\n+ New line".to_string(),
            content_excluded: false,
            content: Some("Full content of file1.rs".to_string()),
        },
        StagedFile {
            path: "file2.rs".to_string(),
            change_type: ChangeType::Added,
            diff: "+ New file content".to_string(),
            content_excluded: false,
            content: Some("Full content of file2.rs".to_string()),
        },
    ];

    context
}

// Test case for small token limit to ensure diffs and commits are prioritized over full content
#[tokio::test]
async fn test_token_optimizer_prioritize_diffs_and_commits() {
    let mut context = create_test_context();

    let config = create_test_config();
    let optimizer = TokenOptimizer::new(15, config).expect("Failed to initialize token optimizer");
    println!(
        "Original token count: {}",
        count_total_tokens(&context, &optimizer)
    );

    let _ = optimizer.optimize_context(&mut context).await;

    print_debug_info(&context, &optimizer);

    let total_tokens = count_total_tokens(&context, &optimizer);
    assert!(
        total_tokens <= 15,
        "Total tokens ({total_tokens}) exceeds limit of 15"
    );

    // File diffs should be fully represented or truncated last
    for file in &context.staged_files {
        assert!(
            optimizer.count_tokens(&file.diff) <= 15,
            "File diff should be within token limit"
        );
    }

    // Check commit messages, ensuring that only the last commit might be truncated
    for (i, commit) in context.recent_commits.iter().enumerate() {
        let commit_tokens = optimizer.count_tokens(&commit.message);
        if i == context.recent_commits.len() - 1 {
            // Only the last commit should potentially be truncated
            assert!(
                commit.message.ends_with('…') || commit_tokens <= 1,
                "Last commit message should be truncated or be at most 1 character long"
            );
        } else {
            // Earlier commits should not be truncated unless there's a severe limit
            assert!(
                commit_tokens <= 15,
                "Earlier commit messages should fit within token limits"
            );
        }
    }

    // Full file content should be the lowest priority and truncated first if needed
    for file in &context.staged_files {
        if let Some(content) = &file.content {
            assert!(
                content.ends_with('…') || optimizer.count_tokens(content) <= 1,
                "Full file content should be truncated or be at most 1 character long"
            );
        }
    }
}

// Test case for large token limit to ensure no content is truncated
#[tokio::test]
async fn test_token_optimizer_large_limit_with_full_content() {
    let mut context = create_test_context();
    let config = create_test_config();
    let optimizer =
        TokenOptimizer::new(1000, config).expect("Failed to initialize token optimizer");

    let _ = optimizer.optimize_context(&mut context).await;

    let total_tokens = count_total_tokens(&context, &optimizer);
    assert!(
        total_tokens <= 1000,
        "Total tokens ({total_tokens}) exceeds limit of 1000"
    );

    // No truncation should occur, especially in file diffs and full content
    for file in &context.staged_files {
        assert!(
            !file.diff.ends_with('…'),
            "File diff should not be truncated"
        );
        if let Some(content) = &file.content {
            assert!(
                !content.ends_with('…'),
                "Full file content should not be truncated"
            );
        }
    }

    for commit in &context.recent_commits {
        assert!(
            !commit.message.ends_with('…'),
            "Commit message should not be truncated"
        );
    }
}

// Helper function to print debug information
fn print_debug_info(context: &CommitContext, optimizer: &TokenOptimizer) {
    if !DEBUG {
        return;
    }
    println!("Commits: {}", context.recent_commits.len());
    for (i, commit) in context.recent_commits.iter().enumerate() {
        let tokens = optimizer.count_tokens(&commit.message);
        println!("Commit {}: '{}' ({} tokens)", i, commit.message, tokens);
    }
    println!("Staged files: {}", context.staged_files.len());
    for (i, file) in context.staged_files.iter().enumerate() {
        let diff_tokens = optimizer.count_tokens(&file.diff);
        println!(
            "Staged file {}: '{}' ({} tokens)",
            i, file.diff, diff_tokens
        );
        if let Some(content) = &file.content {
            let content_tokens = optimizer.count_tokens(content);
            println!("Full content {i}: '{content}' ({content_tokens} tokens)");
        }
    }
}

#[tokio::test]
async fn test_token_optimizer_realistic_limit() {
    let mut context = create_test_context_with_large_data(); // Function that creates the test data
    let config = create_test_config();
    let optimizer =
        TokenOptimizer::new(2000, config).expect("Failed to initialize token optimizer");

    println!(
        "Test token count: {}",
        count_total_tokens(&context, &optimizer)
    );

    // Apply the optimizer to bring the token count within the limit
    let _ = optimizer.optimize_context(&mut context).await;

    // Debugging print to verify the final token count
    let total_tokens = count_total_tokens(&context, &optimizer);
    println!("Total tokens after optimization: {total_tokens}");

    // Assert that the total tokens do not exceed the limit
    assert!(
        total_tokens <= 2000,
        "Total tokens ({total_tokens}) exceeds limit of 2000"
    );

    // Verify that the diffs are prioritized and potentially truncated last
    for file in &context.staged_files {
        let diff_tokens = optimizer.count_tokens(&file.diff);
        if let Some(content) = &file.content {
            let content_tokens = optimizer.count_tokens(content);
            assert!(
                content_tokens <= 2000 - diff_tokens,
                "Full file content should be truncated first if necessary"
            );
        }
        assert!(
            diff_tokens <= 2000,
            "File diff should be within the token limit after truncation"
        );
    }

    // Check that commit messages are truncated if necessary, prioritizing diffs
    for (i, commit) in context.recent_commits.iter().enumerate() {
        let commit_tokens = optimizer.count_tokens(&commit.message);
        if i == context.recent_commits.len() - 1 {
            assert!(
                commit.message.ends_with('…') || commit_tokens <= 1,
                "Last commit message should be truncated if necessary"
            );
        } else {
            assert!(
                commit_tokens <= 2000,
                "Earlier commit messages should fit within token limits"
            );
        }
    }
}

// Helper function to create realistic large test data
fn create_test_context_with_large_data() -> CommitContext {
    let large_diff = "- Old line\n+ New line\n".repeat(200); // 200 repetitions to simulate a large diff
    let large_content = "Full content of the file\n".repeat(200); // Large full file content
    let large_commit_message =
        "Implemented a large feature that touches many parts of the codebase".repeat(20); // Large commit message

    CommitContext {
        branch: "main".to_string(),
        recent_commits: vec![
            RecentCommit {
                hash: "abc123".to_string(),
                message: large_commit_message.clone(),
                author: "Test Author".to_string(),
                timestamp: "2023-01-01 00:00:00".to_string(),
            },
            RecentCommit {
                hash: "def456".to_string(),
                message: large_commit_message.clone(),
                author: "Test Author".to_string(),
                timestamp: "2023-01-02 00:00:00".to_string(),
            },
            RecentCommit {
                hash: "ghi789".to_string(),
                message: large_commit_message,
                author: "Test Author".to_string(),
                timestamp: "2023-01-03 00:00:00".to_string(),
            },
        ],
        staged_files: vec![
            StagedFile {
                path: "file1.rs".to_string(),
                change_type: ChangeType::Modified,
                diff: large_diff.clone(),
                content_excluded: false,
                content: Some(large_content.clone()),
            },
            StagedFile {
                path: "file2.rs".to_string(),
                change_type: ChangeType::Added,
                diff: large_diff,
                content_excluded: false,
                content: Some(large_content),
            },
        ],

        user_name: "Test User".to_string(),
        user_email: "test@example.com".to_string(),
        author_history: vec![
            "feat: implement large feature with many changes".to_string(),
            "fix: resolve performance issue in data processing".to_string(),
        ],
    }
}

// Helper function to count total tokens
fn count_total_tokens(context: &CommitContext, optimizer: &TokenOptimizer) -> usize {
    let commit_tokens: usize = context
        .recent_commits
        .iter()
        .map(|c| optimizer.count_tokens(&c.message))
        .sum();
    let staged_tokens: usize = context
        .staged_files
        .iter()
        .map(|f| {
            optimizer.count_tokens(&f.diff)
                + f.content.as_ref().map_or(0, |c| optimizer.count_tokens(c))
        })
        .sum();
    commit_tokens + staged_tokens
}

// Test importance-weighted token distribution with known importance scores
#[tokio::test]
async fn test_importance_weighted_token_distribution() {
    let mut context = CommitContext {
        branch: "main".to_string(),
        recent_commits: vec![
            RecentCommit {
                hash: "abc123".to_string(),
                message: "Small commit message".to_string(), // ~4 tokens
                author: "Test Author".to_string(),
                timestamp: "2023-01-01 00:00:00".to_string(),
            },
            RecentCommit {
                hash: "def456".to_string(),
                message: "Medium commit message with more details".to_string(), // ~7 tokens
                author: "Test Author".to_string(),
                timestamp: "2023-01-02 00:00:00".to_string(),
            },
        ],
        staged_files: vec![
            StagedFile {
                path: "small_file.rs".to_string(),
                change_type: ChangeType::Modified,
                diff: "- old\n+ new".to_string(), // ~4 tokens
                content_excluded: false,
                content: Some("short content".to_string()), // ~3 tokens
            },
            StagedFile {
                path: "large_file.rs".to_string(),
                change_type: ChangeType::Added,
                diff: "+ large diff content\n".repeat(10), // ~30 tokens
                content_excluded: false,
                content: Some("large file content\n".repeat(10)), // ~30 tokens
            },
        ],
        user_name: "Test User".to_string(),
        user_email: "test@example.com".to_string(),
        author_history: vec![],
    };

    let config = create_test_config();
    let optimizer = TokenOptimizer::new(50, config).expect("Failed to initialize token optimizer");

    let original_tokens = count_total_tokens(&context, &optimizer);
    assert!(
        original_tokens > 50,
        "Test setup should have more than 50 tokens"
    );

    let _ = optimizer.optimize_context(&mut context).await;

    let final_tokens = count_total_tokens(&context, &optimizer);
    assert!(
        final_tokens <= 50,
        "Final token count should not exceed limit of 50"
    );

    // The large file should get more tokens allocated due to higher importance (larger diff)
    let small_file_tokens = optimizer.count_tokens(&context.staged_files[0].diff)
        + context.staged_files[0]
            .content
            .as_ref()
            .map_or(0, |c| optimizer.count_tokens(c));
    let large_file_tokens = optimizer.count_tokens(&context.staged_files[1].diff)
        + context.staged_files[1]
            .content
            .as_ref()
            .map_or(0, |c| optimizer.count_tokens(c));

    // Large file should have more tokens allocated due to higher importance
    assert!(
        large_file_tokens >= small_file_tokens,
        "Large file should get more tokens due to higher importance"
    );
}

// Test that items are processed in importance order
#[tokio::test]
async fn test_importance_order_processing() {
    let mut context = CommitContext {
        branch: "main".to_string(),
        recent_commits: vec![
            RecentCommit {
                hash: "abc123".to_string(),
                message: "First commit - most important".to_string(),
                author: "Test Author".to_string(),
                timestamp: "2023-01-01 00:00:00".to_string(),
            },
            RecentCommit {
                hash: "def456".to_string(),
                message: "Second commit - less important".to_string(),
                author: "Test Author".to_string(),
                timestamp: "2023-01-02 00:00:00".to_string(),
            },
        ],
        staged_files: vec![
            StagedFile {
                path: "important_file.rs".to_string(),
                change_type: ChangeType::Modified,
                diff: "- old\n+ new\n".repeat(5), // 15 tokens - high importance
                content_excluded: false,
                content: Some("important content".to_string()),
            },
            StagedFile {
                path: "less_important_file.rs".to_string(),
                change_type: ChangeType::Modified,
                diff: "- old\n+ new".to_string(), // 4 tokens - low importance
                content_excluded: false,
                content: Some("less important content".to_string()),
            },
        ],
        user_name: "Test User".to_string(),
        user_email: "test@example.com".to_string(),
        author_history: vec![],
    };

    let config = create_test_config();
    let optimizer = TokenOptimizer::new(30, config).expect("Failed to initialize token optimizer");

    let _ = optimizer.optimize_context(&mut context).await;

    let final_tokens = count_total_tokens(&context, &optimizer);
    assert!(final_tokens <= 30, "Should not exceed token limit");

    // The more important file (larger diff) should retain more content
    let important_file_tokens = optimizer.count_tokens(&context.staged_files[0].diff)
        + context.staged_files[0]
            .content
            .as_ref()
            .map_or(0, |c| optimizer.count_tokens(c));
    let less_important_file_tokens = optimizer.count_tokens(&context.staged_files[1].diff)
        + context.staged_files[1]
            .content
            .as_ref()
            .map_or(0, |c| optimizer.count_tokens(c));

    // Important file should have more tokens allocated
    assert!(
        important_file_tokens >= less_important_file_tokens,
        "More important file should retain more tokens"
    );
}

// Test edge case with empty context
#[tokio::test]
async fn test_importance_weighted_empty_context() {
    let mut context = CommitContext {
        branch: "main".to_string(),
        recent_commits: vec![],
        staged_files: vec![],
        user_name: "Test User".to_string(),
        user_email: "test@example.com".to_string(),
        author_history: vec![],
    };

    let config = create_test_config();
    let optimizer = TokenOptimizer::new(100, config).expect("Failed to initialize token optimizer");

    let result = optimizer.optimize_context(&mut context).await;
    assert!(result.is_ok(), "Should handle empty context without error");

    let final_tokens = count_total_tokens(&context, &optimizer);
    assert_eq!(final_tokens, 0, "Empty context should have zero tokens");
}

// Test importance scoring for commits (position factor)
#[tokio::test]
async fn test_commit_importance_position_factor() {
    let mut context = CommitContext {
        branch: "main".to_string(),
        recent_commits: vec![
            RecentCommit {
                hash: "first".to_string(),
                message: "First commit".to_string(), // Should have highest importance (position 0)
                author: "Test Author".to_string(),
                timestamp: "2023-01-01 00:00:00".to_string(),
            },
            RecentCommit {
                hash: "second".to_string(),
                message: "Second commit".to_string(), // Should have lower importance (position 1)
                author: "Test Author".to_string(),
                timestamp: "2023-01-02 00:00:00".to_string(),
            },
            RecentCommit {
                hash: "third".to_string(),
                message: "Third commit".to_string(), // Should have lowest importance (position 2)
                author: "Test Author".to_string(),
                timestamp: "2023-01-03 00:00:00".to_string(),
            },
        ],
        staged_files: vec![],
        user_name: "Test User".to_string(),
        user_email: "test@example.com".to_string(),
        author_history: vec![],
    };

    let config = create_test_config();
    let optimizer = TokenOptimizer::new(10, config).expect("Failed to initialize token optimizer");

    let _ = optimizer.optimize_context(&mut context).await;

    // With limited tokens, later commits should be truncated more
    // First commit should be fully preserved, later ones truncated
    let first_commit_tokens = optimizer.count_tokens(&context.recent_commits[0].message);
    let second_commit_tokens = optimizer.count_tokens(&context.recent_commits[1].message);
    let third_commit_tokens = optimizer.count_tokens(&context.recent_commits[2].message);

    assert!(
        first_commit_tokens >= second_commit_tokens,
        "First commit should have more tokens than second"
    );
    assert!(
        second_commit_tokens >= third_commit_tokens,
        "Second commit should have more tokens than third"
    );
}

// Test content relevance factor for staged files
#[tokio::test]
async fn test_content_relevance_factor() {
    let mut context = CommitContext {
        branch: "main".to_string(),
        recent_commits: vec![],
        staged_files: vec![
            StagedFile {
                path: "staged_file.rs".to_string(),
                change_type: ChangeType::Modified,
                diff: "- old\n+ new".to_string(),
                content_excluded: false,
                content: Some("This is staged file content".to_string()),
            },
            StagedFile {
                path: "unstaged_file.rs".to_string(),
                change_type: ChangeType::Modified,
                diff: "- old\n+ new".to_string(),
                content_excluded: false,
                content: Some("This is unstaged file content".to_string()),
            },
        ],
        user_name: "Test User".to_string(),
        user_email: "test@example.com".to_string(),
        author_history: vec![],
    };

    let config = create_test_config();
    let optimizer = TokenOptimizer::new(15, config).expect("Failed to initialize token optimizer");

    let _ = optimizer.optimize_context(&mut context).await;

    // Both files should have their diffs preserved, but content might be truncated
    // Since both are in staged_files, they should have equal relevance (relevance_factor = 1.0)
    let staged_content_preserved = context.staged_files[0].content.is_some();
    let unstaged_content_preserved = context.staged_files[1].content.is_some();

    // With token limit, some content might be cleared, but both should be treated equally
    // since they're both in the staged_files vector
    assert_eq!(
        staged_content_preserved, unstaged_content_preserved,
        "Both staged files should have same content preservation treatment"
    );
}

// Test sentence boundary truncation to avoid mid-sentence cuts
#[tokio::test]
async fn test_sentence_boundary_truncation() {
    let config = create_test_config();
    let optimizer =
        TokenOptimizer::new(1000, config).expect("Failed to initialize token optimizer");

    // Test text with clear sentence boundaries
    let test_text = "This is the first sentence. This is the second sentence! This is the third sentence? This is the fourth sentence.";

    // Truncate to a small number of tokens that would cut mid-sentence without boundary detection
    let result = optimizer
        .truncate_string(test_text, 15)
        .expect("Truncation should succeed");

    // Should end with a complete sentence, not cut mid-sentence
    assert!(result.ends_with("…"), "Should end with ellipsis");

    // Should not cut in the middle of a sentence
    let text_without_ellipsis = result.trim_end_matches('…');
    assert!(
        text_without_ellipsis.ends_with('.')
            || text_without_ellipsis.ends_with('!')
            || text_without_ellipsis.ends_with('?')
            || text_without_ellipsis.is_empty(),
        "Should end at sentence boundary, got: {text_without_ellipsis}"
    );

    // Test with text that has no good sentence boundaries
    let no_sentences =
        "This is a long continuous text without proper sentence endings it just keeps going";
    let result2 = optimizer
        .truncate_string(no_sentences, 10)
        .expect("Truncation should succeed");
    assert!(
        result2.ends_with("…"),
        "Should still add ellipsis even without sentence boundaries"
    );
}

// Test proportional allocation with extreme importance differences
#[tokio::test]
async fn test_extreme_importance_differences() {
    let mut context = CommitContext {
        branch: "main".to_string(),
        recent_commits: vec![RecentCommit {
            hash: "important".to_string(),
            message: "Very important commit with lots of details that should be preserved"
                .to_string(),
            author: "Test Author".to_string(),
            timestamp: "2023-01-01 00:00:00".to_string(),
        }],
        staged_files: vec![StagedFile {
            path: "critical_diff.rs".to_string(),
            change_type: ChangeType::Modified,
            diff: "+ critical change\n".repeat(20), // 40 tokens - very important
            content_excluded: false,
            content: Some("small content".to_string()), // 3 tokens - less important
        }],
        user_name: "Test User".to_string(),
        user_email: "test@example.com".to_string(),
        author_history: vec![],
    };

    let config = create_test_config();
    let optimizer = TokenOptimizer::new(25, config).expect("Failed to initialize token optimizer");

    let _ = optimizer.optimize_context(&mut context).await;

    let final_tokens = count_total_tokens(&context, &optimizer);
    assert!(final_tokens <= 25, "Should not exceed token limit");

    // The large diff should be significantly truncated, while smaller items might be preserved
    let diff_tokens = optimizer.count_tokens(&context.staged_files[0].diff);
    let _content_tokens = context.staged_files[0]
        .content
        .as_ref()
        .map_or(0, |c| optimizer.count_tokens(c));
    let commit_tokens = optimizer.count_tokens(&context.recent_commits[0].message);

    // Diff should be truncated the most due to its size
    assert!(diff_tokens < 40, "Large diff should be truncated");
    // Smaller items should be relatively preserved
    assert!(commit_tokens > 0, "Commit should retain some tokens");
}
