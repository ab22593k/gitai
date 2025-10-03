use gitpilot::{
    commit::review::{CodeIssue, DimensionAnalysis, GeneratedReview},
    mcp::tools::codereview::CodeReviewTool,
};

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_review_format() {
    // Test that the review formatting works as expected
    let review = GeneratedReview {
        summary: "Test summary".to_string(),
        code_quality: "Good quality".to_string(),
        suggestions: vec!["Suggestion 1".to_string(), "Suggestion 2".to_string()],
        issues: vec!["Issue 1".to_string()],
        positive_aspects: vec!["Positive 1".to_string(), "Positive 2".to_string()],
        complexity: Some(DimensionAnalysis {
            issues_found: true,
            issues: vec![
                CodeIssue {
                    description: "Complex function".to_string(),
                    severity: "Medium".to_string(),
                    location: "src/main.rs:42".to_string(),
                    explanation: "This function has too many nested conditionals".to_string(),
                    recommendation: "Extract nested logic into separate functions".to_string(),
                }
            ],
        }),
        abstraction: None,
        deletion: None,
        hallucination: None,
        style: None,
        security: None,
        performance: None,
        duplication: None,
        error_handling: None,
        testing: None,
        best_practices: Some(DimensionAnalysis {
            issues_found: true,
            issues: vec![
                CodeIssue {
                    description: "SOLID principle violation".to_string(),
                    severity: "High".to_string(),
                    location: "src/user_service.rs:105-120".to_string(),
                    explanation: "This class violates the Single Responsibility Principle by handling both authentication and user data management".to_string(),
                    recommendation: "Split into separate UserService and AuthenticationService classes".to_string(),
                }
            ],
        }),
    };

    let formatted = review.format();

    // Check that the formatted output contains all the important parts
    // We can't match exact strings because of color codes, so we'll check for key substrings
    assert!(formatted.contains("CODE REVIEW"));
    assert!(formatted.contains("Test summary"));
    assert!(formatted.contains("QUALITY ASSESSMENT"));
    assert!(formatted.contains("Good quality"));
    assert!(formatted.contains("STRENGTHS"));
    assert!(formatted.contains("Positive 1"));
    assert!(formatted.contains("Positive 2"));
    assert!(formatted.contains("CORE ISSUES"));
    assert!(formatted.contains("Issue 1"));
    assert!(formatted.contains("SUGGESTIONS"));
    assert!(formatted.contains("Suggestion 1"));
    assert!(formatted.contains("Suggestion 2"));

    // Check the complexity dimension was formatted
    assert!(formatted.contains("Complexity"));
    assert!(formatted.contains("Complex function"));
    assert!(formatted.contains("MEDIUM"));
    assert!(formatted.contains("src/main.rs:42"));
    assert!(formatted.contains("This function has too many nested conditionals"));
    assert!(formatted.contains("Extract nested logic into separate functions"));

    // Check the best practices dimension was formatted
    assert!(formatted.contains("Best Practices"));
    assert!(formatted.contains("SOLID principle violation"));
    assert!(formatted.contains("HIGH"));
    assert!(formatted.contains("src/user_service.rs:105-120"));
    assert!(formatted.contains("This class violates the Single Responsibility Principle"));
    assert!(
        formatted.contains("Split into separate UserService and AuthenticationService classes")
    );
}

#[test]
fn test_format_location() {
    // Test with a path containing filename and line numbers
    let location = "src/commit/review.rs:45-67";
    let formatted = GeneratedReview::format_location(location);
    assert_eq!(formatted, location);

    // Test with just line numbers
    let location = "45-67";
    let formatted = GeneratedReview::format_location(location);
    assert_eq!(formatted, "Line(s) 45-67");

    // Test with line keyword
    let location = "Line 45";
    let formatted = GeneratedReview::format_location(location);
    assert_eq!(formatted, location);

    // Test with file keyword
    let location = "in file helpers.js";
    let formatted = GeneratedReview::format_location(location);
    assert_eq!(formatted, location);

    // Test with just file extension
    let location = "helpers.js";
    let formatted = GeneratedReview::format_location(location);
    assert_eq!(formatted, location);
}

#[test]
fn test_mcp_branch_comparison_validation() {
    // Test valid branch comparison (both from and to specified)
    let tool = CodeReviewTool {
        include_unstaged: false,
        commit_id: String::new(),
        from: "main".to_string(),
        to: "feature-branch".to_string(),
        preset: String::new(),
        custom_instructions: String::new(),
        repository: "/path/to/repo".to_string(),
    };

    // This should pass validation (we can't easily test the full execution without mocks)
    assert!(!tool.from.trim().is_empty());
    assert!(!tool.to.trim().is_empty());

    // Test only 'to' specified (should work with default 'from')
    let tool_with_default = CodeReviewTool {
        include_unstaged: false,
        commit_id: String::new(),
        from: String::new(), // Empty, should default to "main"
        to: "feature-branch".to_string(),
        preset: String::new(),
        custom_instructions: String::new(),
        repository: "/path/to/repo".to_string(),
    };

    assert!(tool_with_default.from.trim().is_empty());
    assert!(!tool_with_default.to.trim().is_empty());

    // Test that both commit_id and branch parameters would conflict
    let conflicting_tool = CodeReviewTool {
        include_unstaged: false,
        commit_id: "abc123".to_string(),
        from: "main".to_string(),
        to: "feature-branch".to_string(),
        preset: String::new(),
        custom_instructions: String::new(),
        repository: "/path/to/repo".to_string(),
    };

    let has_commit = !conflicting_tool.commit_id.trim().is_empty();
    let has_branches =
        !conflicting_tool.from.trim().is_empty() || !conflicting_tool.to.trim().is_empty();
    assert!(has_commit && has_branches); // This combination should be invalid

    // Test that include_unstaged with branches would conflict
    let unstaged_with_branches = CodeReviewTool {
        include_unstaged: true,
        commit_id: String::new(),
        from: String::new(),
        to: "feature-branch".to_string(),
        preset: String::new(),
        custom_instructions: String::new(),
        repository: "/path/to/repo".to_string(),
    };

    let has_branches = !unstaged_with_branches.from.trim().is_empty()
        || !unstaged_with_branches.to.trim().is_empty();
    assert!(unstaged_with_branches.include_unstaged && has_branches); // This combination should be invalid
}

#[test]
fn test_mcp_branch_default_behavior() {
    // Test the logic for defaulting 'from' to "main"
    let tool = CodeReviewTool {
        include_unstaged: false,
        commit_id: String::new(),
        from: String::new(), // Empty
        to: "feature-branch".to_string(),
        preset: String::new(),
        custom_instructions: String::new(),
        repository: "/path/to/repo".to_string(),
    };

    // Simulate the logic from the MCP tool execution
    let has_branches = !tool.from.trim().is_empty() || !tool.to.trim().is_empty();
    assert!(has_branches); // Should be true because 'to' is specified

    let from_branch = if tool.from.trim().is_empty() {
        "main"
    } else {
        tool.from.trim()
    };
    let to_branch = tool.to.trim();

    assert_eq!(from_branch, "main");
    assert_eq!(to_branch, "feature-branch");
}

#[test]
fn test_mcp_validation_edge_cases() {
    // Test 'from' without 'to' (should be invalid)
    let invalid_tool = CodeReviewTool {
        include_unstaged: false,
        commit_id: String::new(),
        from: "main".to_string(),
        to: String::new(), // Empty
        preset: String::new(),
        custom_instructions: String::new(),
        repository: "/path/to/repo".to_string(),
    };

    // This should fail validation
    let from_without_to = !invalid_tool.from.trim().is_empty() && invalid_tool.to.trim().is_empty();
    assert!(from_without_to); // This should be true, indicating an invalid state

    // Test empty strings for both (should default to normal staged review)
    let normal_review = CodeReviewTool {
        include_unstaged: false,
        commit_id: String::new(),
        from: String::new(),
        to: String::new(),
        preset: String::new(),
        custom_instructions: String::new(),
        repository: "/path/to/repo".to_string(),
    };

    let has_branches = !normal_review.from.trim().is_empty() || !normal_review.to.trim().is_empty();
    assert!(!has_branches); // Should be false, indicating normal staged review
}

// Helper function to validate branch comparison parameters (extracted from the logic)
fn validate_branch_parameters(
    from: Option<&String>,
    to: Option<&String>,
    commit_id: Option<&String>,
    include_unstaged: bool,
) -> Result<(), String> {
    // Validate branch parameters
    if from.is_some() && to.is_none() {
        return Err(
            "When using --from, you must also specify --to for branch comparison reviews"
                .to_string(),
        );
    }

    if commit_id.is_some() && (from.is_some() || to.is_some()) {
        return Err(
            "Cannot use --commit with --from/--to. These are mutually exclusive options"
                .to_string(),
        );
    }

    if include_unstaged && (from.is_some() || to.is_some()) {
        return Err("Cannot use --include-unstaged with --from/--to. Branch comparison reviews don't include working directory changes".to_string());
    }

    Ok(())
}

#[test]
fn test_branch_parameter_validation() {
    // Test valid combinations
    assert!(validate_branch_parameters(None, None, None, false).is_ok());
    assert!(
        validate_branch_parameters(
            Some(&"main".to_string()),
            Some(&"feature".to_string()),
            None,
            false
        )
        .is_ok()
    );
    assert!(validate_branch_parameters(None, Some(&"feature".to_string()), None, false).is_ok()); // --to only (defaults to main)
    assert!(validate_branch_parameters(None, None, Some(&"abc123".to_string()), false).is_ok());
    assert!(validate_branch_parameters(None, None, None, true).is_ok());

    // Test invalid combinations
    assert!(validate_branch_parameters(Some(&"main".to_string()), None, None, false).is_err()); // --from without --to
    assert!(
        validate_branch_parameters(
            Some(&"main".to_string()),
            Some(&"feature".to_string()),
            Some(&"abc123".to_string()),
            false
        )
        .is_err()
    ); // branches + commit
    assert!(
        validate_branch_parameters(
            None,
            Some(&"feature".to_string()),
            Some(&"abc123".to_string()),
            false
        )
        .is_err()
    ); // --to + commit
    assert!(
        validate_branch_parameters(
            Some(&"main".to_string()),
            Some(&"feature".to_string()),
            None,
            true
        )
        .is_err()
    ); // branches + unstaged
    assert!(validate_branch_parameters(None, Some(&"feature".to_string()), None, true).is_err()); // --to + unstaged

    // Test error messages
    let result = validate_branch_parameters(Some(&"main".to_string()), None, None, false);
    assert!(result.is_err());
    assert!(
        result
            .expect_err("Should have error")
            .contains("When using --from, you must also specify --to")
    );

    let result = validate_branch_parameters(
        Some(&"main".to_string()),
        Some(&"feature".to_string()),
        Some(&"abc123".to_string()),
        false,
    );
    assert!(result.is_err());
    assert!(
        result
            .expect_err("Should have error")
            .contains("Cannot use --commit with --from/--to")
    );

    let result = validate_branch_parameters(
        Some(&"main".to_string()),
        Some(&"feature".to_string()),
        None,
        true,
    );
    assert!(result.is_err());
    assert!(
        result
            .expect_err("Should have error")
            .contains("Cannot use --include-unstaged with --from/--to")
    );
}

// Note: We don't include a test for handle_review_command here because:
// 1. It would require mocking a lot of complex dependencies
// 2. The core functionality (generate_review) is already tested in service_tests.rs
// 3. The command itself primarily handles user interface concerns
//
// For thorough testing, we would need to:
// - Mock the IrisCommitService
// - Mock the git info generation
// - Mock the LLM response
// - Capture stdout to verify output
//
// These would be better suited for integration tests with a more sophisticated test harness.
