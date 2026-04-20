//! Prompt builder with fluent API.
//!
//! Composes prompt sections into a complete system or user prompt.

use schemars::JsonSchema;
use serde::Serialize;

use crate::config::Config;
use crate::llm::context::CommitContext;

use super::sections::{
    DataContextSection, GuidelinesSection, OutputSchemaSection, PersonaSection, PromptSection,
    TaskSection, UserInstructionsSection,
};

/// Builder for constructing system prompts from composable sections.
pub struct PromptBuilder {
    sections: Vec<String>,
}

impl PromptBuilder {
    /// Start building a system prompt.
    #[must_use]
    pub fn system() -> Self {
        Self {
            sections: Vec::new(),
        }
    }

    /// Add a persona section.
    #[must_use]
    pub fn with_persona(mut self, persona: &str) -> Self {
        self.sections.push(PersonaSection::new(persona).render());
        self
    }

    /// Add a task section.
    #[must_use]
    pub fn with_task(mut self, task: &str) -> Self {
        self.sections.push(TaskSection::new(task).render());
        self
    }

    /// Add operational guidelines.
    #[must_use]
    pub fn with_guidelines(mut self, guidelines: &[&str]) -> Self {
        self.sections
            .push(GuidelinesSection::new(guidelines).render());
        self
    }

    /// Add user instructions from config.
    #[must_use]
    pub fn with_user_instructions(mut self, config: &Config) -> Self {
        self.sections
            .push(UserInstructionsSection::new(config).render());
        self
    }

    /// Add output specification with JSON schema for type `T`.
    #[must_use]
    pub fn with_output_schema<T: JsonSchema + Serialize>(mut self) -> Self {
        self.sections.push(OutputSchemaSection::<T>::new().render());
        self
    }

    /// Add data context from a commit context.
    #[must_use]
    pub fn with_data_context(mut self, ctx: &CommitContext) -> Self {
        self.sections.push(DataContextSection::new(ctx).render());
        self
    }

    /// Build the complete prompt by concatenating all sections.
    #[must_use]
    pub fn build(self) -> String {
        self.sections.join("\n")
    }
}
