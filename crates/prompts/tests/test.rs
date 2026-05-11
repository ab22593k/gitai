use prompts::{
    builder,
    sections::{
        DataContextSection, GuidelinesSection, OutputSchemaSection, PersonaSection, PromptSection,
        TaskSection,
    },
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
struct TestSchema {
    pub title: String,
    pub message: String,
}

#[test]
fn test_persona_section_renders_header_and_content() {
    let section = PersonaSection::new("Test persona content");
    let output = section.render();
    assert!(output.contains("# PERSONA"));
    assert!(output.contains("Test persona content"));
}

#[test]
fn test_task_section_renders_header_and_content() {
    let section = TaskSection::new("Generate a commit message");
    let output = section.render();
    assert!(output.contains("# TASK"));
    assert!(output.contains("Generate a commit message"));
}

#[test]
fn test_guidelines_section_renders_numbered_list() {
    let guidelines = vec!["Use imperative mood", "Limit subject to 72 characters"];
    let section = GuidelinesSection::new(&guidelines);
    let output = section.render();
    assert!(output.contains("# OPERATIONAL GUIDELINES"));
    assert!(output.contains("1. **"));
    assert!(output.contains("Use imperative mood"));
}

#[test]
fn test_output_schema_section_renders_json_schema() {
    let section = OutputSchemaSection::<TestSchema>::new();
    let output = section.render();
    assert!(output.contains("# OUTPUT SPECIFICATION"));
    assert!(output.contains("```json"));
    assert!(output.contains("TestSchema"));
}

#[test]
fn test_data_context_section_renders_branch() {
    let section = DataContextSection::new("feature/test", "src/main.rs (Modified)", "");
    let output = section.render();
    assert!(output.contains("- **Branch:** `feature/test`"));
}

#[test]
fn test_data_context_section_renders_staged_files() {
    let section = DataContextSection::new("main", "src/main.rs (Modified)", "");
    let output = section.render();
    assert!(output.contains("src/main.rs (Modified)"));
}

#[test]
fn test_data_context_section_renders_contextual_history() {
    let section = DataContextSection::new("main", "", "abc123 - Previous commit");
    let output = section.render();
    assert!(output.contains("- **Contextual History:**"));
    assert!(output.contains("Previous commit"));
}

#[test]
fn test_prompt_builder_with_persona() {
    let output = builder::PromptBuilder::system()
        .with_persona("Linux Kernel Maintainer")
        .build();
    assert!(output.contains("# PERSONA"));
    assert!(output.contains("Linux Kernel Maintainer"));
}

#[test]
fn test_prompt_builder_with_task() {
    let output = builder::PromptBuilder::system()
        .with_task("Generate commit message")
        .build();
    assert!(output.contains("# TASK"));
    assert!(output.contains("Generate commit message"));
}

#[test]
fn test_prompt_builder_with_guidelines() {
    let output = builder::PromptBuilder::system()
        .with_guidelines(&["Rule 1", "Rule 2"])
        .build();
    assert!(output.contains("# OPERATIONAL GUIDELINES"));
    assert!(output.contains("Rule 1"));
    assert!(output.contains("Rule 2"));
}

#[test]
fn test_prompt_builder_with_user_instructions() {
    let output = builder::PromptBuilder::system()
        .with_user_instructions("Custom instructions")
        .build();
    assert!(output.contains("# USER INSTRUCTIONS"));
}

#[test]
fn test_prompt_builder_with_output_schema() {
    let output = builder::PromptBuilder::system()
        .with_output_schema::<TestSchema>()
        .build();
    assert!(output.contains("# OUTPUT SPECIFICATION"));
    assert!(output.contains("```json"));
}

#[test]
fn test_prompt_builder_chained_sections() {
    let output = builder::PromptBuilder::system()
        .with_persona("Test persona")
        .with_task("Test task")
        .with_guidelines(&["Rule 1"])
        .build();
    let persona_pos = output.find("# PERSONA").expect("PERSONA section missing");
    let task_pos = output.find("# TASK").expect("TASK section missing");
    let guidelines_pos = output
        .find("# OPERATIONAL GUIDELINES")
        .expect("GUIDELINES missing");
    assert!(persona_pos < task_pos, "PERSONA should come before TASK");
    assert!(
        task_pos < guidelines_pos,
        "TASK should come before GUIDELINES"
    );
}

#[test]
fn test_prompt_builder_full_commit_system_prompt() {
    let builder_output = builder::PromptBuilder::system()
        .with_persona(
            "You are a Principal Linux Kernel Maintainer. You are technically rigorous, demanding, \
             and believe that a commit message is a permanent piece of technical documentation. \
             You expect developers to explain *why* a change is necessary with absolute precision.",
        )
        .with_task(
            "Generate a technical commit message for a high-stakes mailing list. The message must \
             provide a clear technical narrative explaining the Problem, Solution, and Reasoning.",
        )
        .with_output_schema::<TestSchema>()
        .build();
    assert!(builder_output.contains("# PERSONA"));
    assert!(builder_output.contains("# TASK"));
    assert!(builder_output.contains("# OUTPUT SPECIFICATION"));
    assert!(builder_output.contains("```json"));
    assert!(builder_output.contains("Linux Kernel Maintainer"));
}
