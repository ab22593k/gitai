//! Tests for the MCP integration

#[cfg(test)]
mod tests {
    use gitpilot::{
        config::Config,
        git::GitRepo,
        mcp::tools::{GitIrisTools, PrTool, ReleaseNotesTool, utils::GitIrisTool},
    };

    use rmcp::model::{Content, RawContent};
    use serde_json::{Map, Value, json};
    use std::sync::Arc;

    // Helper function to create a Map<String, Value> from a JSON object
    fn create_params_map(json_value: &Value) -> Map<String, Value> {
        json_value
            .as_object()
            .expect("JSON value must be an object")
            .clone()
    }

    // Helper function to extract text from Content
    fn get_text_from_content(content: &Content) -> String {
        match &content.raw {
            RawContent::Text(text_content) => text_content.text.clone(),
            _ => panic!("Expected text content"),
        }
    }

    #[tokio::test]
    #[ignore = "Run manually when testing MCP functionality"]
    async fn test_release_notes_tool_direct() {
        // Initialize dependencies
        let git_repo = match GitRepo::new_from_url(None) {
            Ok(repo) => Arc::new(repo),
            Err(e) => {
                eprintln!("Error creating git repo for test: {e}");
                return;
            }
        };

        let config = Config::default();

        // Create the release notes tool instance
        let tool = ReleaseNotesTool {
            from: "HEAD~5".to_string(), // Last 5 commits
            to: "HEAD".to_string(),
            detail_level: "minimal".to_string(),
            custom_instructions: "Keep it brief".to_string(),
            repository: String::new(),
            version_name: String::new(),
        };

        // Execute the tool directly
        match tool.execute(git_repo, config).await {
            Ok(result) => {
                let content = &result.content[0];
                let text = get_text_from_content(content);

                println!(
                    "Release notes content (first 200 chars): {}",
                    text.chars().take(200).collect::<String>()
                );

                assert!(!text.is_empty(), "Release notes should not be empty");
            }
            Err(e) => {
                panic!("Tool execution failed: {e}");
            }
        }
    }

    #[tokio::test]
    #[ignore = "Run manually when testing MCP functionality"]
    async fn test_tools_conversion() {
        // Create parameters for the release notes tool
        let args = create_params_map(&json!({
            "from": "HEAD~5",
            "to": "HEAD",
            "detail_level": "minimal",
            "custom_instructions": "Keep it brief",
            "version_name": ""
        }));

        // Add tool name to parameters for GitIrisTools::try_from
        let mut params = args.clone();
        params.insert(
            "name".to_string(),
            Value::String("git_iris_release_notes".to_string()),
        );

        // Convert to our GitIrisTools enum
        let tool_params =
            GitIrisTools::try_from(params).expect("Failed to convert parameters to GitIrisTools");

        // Verify the tool was created correctly
        match tool_params {
            GitIrisTools::ReleaseNotesTool(tool) => {
                assert_eq!(tool.from, "HEAD~5", "From field not set correctly");
                assert_eq!(tool.to, "HEAD", "To field not set correctly");
                assert_eq!(
                    tool.detail_level, "minimal",
                    "Detail level not set correctly"
                );
                assert_eq!(
                    tool.custom_instructions, "Keep it brief",
                    "Custom instructions not set correctly"
                );
                assert_eq!(tool.version_name, "", "Version name not set correctly");
            }
            GitIrisTools::ChangelogTool(_) => {
                // Not testing this variant in this test
                println!("ChangelogTool variant not being tested in this test");
            }
            GitIrisTools::CommitTool(_) => {
                // Not testing this variant in this test
                println!("CommitTool variant not being tested in this test");
            }
            GitIrisTools::CodeReviewTool(_) => {
                // Not testing this variant in this test
                println!("CodeReviewTool variant not being tested in this test");
            }
            GitIrisTools::PrTool(_) => {
                // Not testing this variant in this test
                println!("PrTool variant not being tested in this test");
            }
        }
    }

    #[tokio::test]
    async fn test_mcp_tools_include_pr_tool() {
        // Test that the PR tool is included in the available tools
        let tools = GitIrisTools::get_tools();

        // Should have 5 tools now (including the new PR tool)
        assert_eq!(tools.len(), 5);

        // Check that git_iris_pr is included
        let pr_tool_exists = tools.iter().any(|tool| tool.name == "git_iris_pr");
        assert!(pr_tool_exists, "git_iris_pr tool should be available");

        // Check that all expected tools are present
        let tool_names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();
        assert!(tool_names.contains(&"git_iris_commit"));
        assert!(tool_names.contains(&"git_iris_code_review"));
        assert!(tool_names.contains(&"git_iris_pr"));
        assert!(tool_names.contains(&"git_iris_changelog"));
        assert!(tool_names.contains(&"git_iris_release_notes"));
    }

    #[test]
    fn test_pr_tool_definition() {
        let tool = PrTool::get_tool_definition();

        assert_eq!(tool.name, "git_iris_pr");
        assert!(tool.description.contains("pull request descriptions"));
        assert!(tool.description.contains("atomic unit"));

        // The input schema should be present and valid
        assert!(
            !tool.input_schema.is_empty(),
            "Input schema should not be empty"
        );
    }

    #[test]
    fn test_pr_tool_serialization() {
        let pr_tool = PrTool {
            from: "main".to_string(),
            to: "feature-branch".to_string(),
            preset: "conventional".to_string(),
            custom_instructions: "Focus on breaking changes".to_string(),
            repository: "/tmp/test-repo".to_string(),
        };

        // Should be able to serialize/deserialize
        let json = serde_json::to_string(&pr_tool).expect("Should serialize");
        let deserialized: PrTool = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(deserialized.from, "main");
        assert_eq!(deserialized.to, "feature-branch");
        assert_eq!(deserialized.preset, "conventional");
        assert_eq!(
            deserialized.custom_instructions,
            "Focus on breaking changes"
        );
        assert_eq!(deserialized.repository, "/tmp/test-repo");
    }
}
