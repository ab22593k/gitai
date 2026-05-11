use schemars::JsonSchema;
use serde::Serialize;
use std::fmt::Write as _;

pub trait PromptSection {
    fn render(&self) -> String;
}

pub struct PersonaSection(String);

impl PersonaSection {
    pub fn new(content: &str) -> Self {
        Self(content.to_string())
    }
}

impl PromptSection for PersonaSection {
    fn render(&self) -> String {
        format!("# PERSONA\n{}\n", self.0)
    }
}

pub struct TaskSection(String);

impl TaskSection {
    pub fn new(content: &str) -> Self {
        Self(content.to_string())
    }
}

impl PromptSection for TaskSection {
    fn render(&self) -> String {
        format!("# TASK\n{}\n", self.0)
    }
}

pub struct GuidelinesSection(Vec<String>);

impl GuidelinesSection {
    pub fn new(guidelines: &[&str]) -> Self {
        Self(guidelines.iter().map(ToString::to_string).collect())
    }
}

impl PromptSection for GuidelinesSection {
    fn render(&self) -> String {
        let mut items = String::new();
        for (i, g) in self.0.iter().enumerate() {
            writeln!(items, "{}. **{g}**", i + 1).expect("String write is infallible");
        }
        format!("# OPERATIONAL GUIDELINES\n\n{items}\n")
    }
}

pub struct OutputSchemaSection<T: JsonSchema + Serialize>(std::marker::PhantomData<T>);

impl<T: JsonSchema + Serialize> Default for OutputSchemaSection<T> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T: JsonSchema + Serialize> OutputSchemaSection<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T: JsonSchema + Serialize> PromptSection for OutputSchemaSection<T> {
    fn render(&self) -> String {
        let schema = schemars::schema_for!(T);
        let schema_str = serde_json::to_string_pretty(&schema).unwrap_or_else(|e| {
            log::warn!("Schema serialization failed: {e}");
            String::new()
        });
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

pub struct DataContextSection {
    branch: String,
    staged_files: String,
    recent_commits: String,
}

impl DataContextSection {
    pub fn new(branch: &str, staged_files: &str, recent_commits: &str) -> Self {
        Self {
            branch: branch.to_string(),
            staged_files: staged_files.to_string(),
            recent_commits: recent_commits.to_string(),
        }
    }
}

impl PromptSection for DataContextSection {
    fn render(&self) -> String {
        let mut parts = vec![String::from("# DATA CONTEXT\n")];

        parts.push(format!("- **Branch:** `{}`\n", self.branch));

        if !self.staged_files.is_empty() {
            parts.push(format!(
                "- **Staged Change List:**\n{}\n",
                self.staged_files
            ));
        }

        if !self.recent_commits.is_empty() {
            parts.push(format!(
                "- **Contextual History:**\n{}\n",
                self.recent_commits
            ));
        }

        parts.join("\n")
    }
}

pub struct UserInstructionsSection(String);

impl UserInstructionsSection {
    pub fn new(instructions: &str) -> Self {
        Self(instructions.to_string())
    }
}

impl PromptSection for UserInstructionsSection {
    fn render(&self) -> String {
        if self.0.is_empty() {
            String::from("# USER INSTRUCTIONS\n\nNo additional instructions provided.\n")
        } else {
            format!("# USER INSTRUCTIONS\n{}\n", self.0)
        }
    }
}
