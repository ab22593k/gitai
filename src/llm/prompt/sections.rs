//! Prompt section abstractions.
//!
//! Each section represents a logical part of a system or user prompt.
//! Sections can be composed to build complete prompts.

use schemars::JsonSchema;
use serde::Serialize;
use std::fmt::Write as _;

use crate::common::get_combined_instructions;
use crate::config::Config;
use crate::llm::context::CommitContext;

/// A single section of a prompt that can be rendered to a string.
pub trait PromptSection {
    /// Render this section to a string.
    fn render(&self) -> String;
}

// -- Concrete Section Implementations --

/// Renders the `# PERSONA` section of a system prompt.
pub struct PersonaSection(String);

impl PersonaSection {
    #[must_use]
    pub fn new(content: &str) -> Self {
        Self(content.to_string())
    }
}

impl PromptSection for PersonaSection {
    fn render(&self) -> String {
        format!("# PERSONA\n{}\n", self.0)
    }
}

/// Renders the `# TASK` section of a system prompt.
pub struct TaskSection(String);

impl TaskSection {
    #[must_use]
    pub fn new(content: &str) -> Self {
        Self(content.to_string())
    }
}

impl PromptSection for TaskSection {
    fn render(&self) -> String {
        format!("# TASK\n{}\n", self.0)
    }
}

/// Renders the `# OPERATIONAL GUIDELINES` section as a numbered list.
pub struct GuidelinesSection(Vec<String>);

impl GuidelinesSection {
    #[must_use]
    pub fn new(guidelines: &[&str]) -> Self {
        Self(guidelines.iter().map(ToString::to_string).collect())
    }
}

impl PromptSection for GuidelinesSection {
    fn render(&self) -> String {
        let mut items = String::new();
        for (i, g) in self.0.iter().enumerate() {
            let _ = writeln!(items, "{}. **{g}**", i + 1);
        }
        format!("# OPERATIONAL GUIDELINES\n\n{items}\n")
    }
}

/// Renders the `# OUTPUT SPECIFICATION` section with a JSON schema for type `T`.
pub struct OutputSchemaSection<T: JsonSchema + Serialize>(std::marker::PhantomData<T>);

impl<T: JsonSchema + Serialize> Default for OutputSchemaSection<T> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T: JsonSchema + Serialize> OutputSchemaSection<T> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T: JsonSchema + Serialize> PromptSection for OutputSchemaSection<T> {
    fn render(&self) -> String {
        let schema = schemars::schema_for!(T);
        let schema_str = serde_json::to_string_pretty(&schema).unwrap_or_default();
        format!(
            "# OUTPUT SPECIFICATION\n\
             \n\
             Your final response MUST be a single, valid JSON object strictly following this schema:\n\
             \n\
             ```json\n\
             {schema_str}\n\
             ```\n\
             \n\
             **CRITICAL:** Output ONLY the JSON. No conversational filler.\n"
        )
    }
}

/// Renders the `# DATA CONTEXT` section from a `CommitContext`.
pub struct DataContextSection<'a>(&'a CommitContext);

impl<'a> DataContextSection<'a> {
    #[must_use]
    pub fn new(ctx: &'a CommitContext) -> Self {
        Self(ctx)
    }
}

impl PromptSection for DataContextSection<'_> {
    fn render(&self) -> String {
        let ctx = self.0;
        let mut parts = vec![String::from("# DATA CONTEXT\n")];

        parts.push(format!("- **Branch:** `{}`\n", ctx.branch));

        if !ctx.staged_files.is_empty() {
            let files: String = ctx
                .staged_files
                .iter()
                .map(|f| format!("  - `{}` ({})", f.path, f.change_type))
                .collect::<Vec<_>>()
                .join("\n");
            parts.push(format!("- **Staged Change List:**\n{files}\n"));
        }

        if !ctx.recent_commits.is_empty() {
            let commits = ctx
                .recent_commits
                .iter()
                .map(|c| {
                    let short = &c.hash[..c.hash.len().min(7)];
                    format!("  - `{short}` {}", c.message)
                })
                .collect::<Vec<_>>()
                .join("\n");
            parts.push(format!("- **Contextual History:**\n{commits}\n"));
        }

        parts.join("\n")
    }
}

/// Renders the `# USER INSTRUCTIONS` section from config.
pub struct UserInstructionsSection<'a>(&'a Config);

impl<'a> UserInstructionsSection<'a> {
    #[must_use]
    pub fn new(config: &'a Config) -> Self {
        Self(config)
    }
}

impl PromptSection for UserInstructionsSection<'_> {
    fn render(&self) -> String {
        let instructions = get_combined_instructions(self.0);
        if instructions.is_empty() {
            String::from("# USER INSTRUCTIONS\n\nNo additional instructions provided.\n")
        } else {
            format!("# USER INSTRUCTIONS\n{instructions}\n")
        }
    }
}
