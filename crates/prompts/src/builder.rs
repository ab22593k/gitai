use schemars::JsonSchema;
use serde::Serialize;

use super::sections::{
    DataContextSection, GuidelinesSection, OutputSchemaSection, PersonaSection, PromptSection,
    TaskSection, UserInstructionsSection,
};

#[must_use]
pub struct PromptBuilder {
    sections: Vec<String>,
}

impl PromptBuilder {
    pub fn system() -> Self {
        Self {
            sections: Vec::new(),
        }
    }

    pub fn with_persona(mut self, persona: &str) -> Self {
        self.sections.push(PersonaSection::new(persona).render());
        self
    }

    pub fn with_task(mut self, task: &str) -> Self {
        self.sections.push(TaskSection::new(task).render());
        self
    }

    pub fn with_guidelines(mut self, guidelines: &[&str]) -> Self {
        self.sections
            .push(GuidelinesSection::new(guidelines).render());
        self
    }

    pub fn with_user_instructions(mut self, instructions: &str) -> Self {
        self.sections
            .push(UserInstructionsSection::new(instructions).render());
        self
    }

    pub fn with_output_schema<T: JsonSchema + Serialize>(mut self) -> Self {
        self.sections.push(OutputSchemaSection::<T>::new().render());
        self
    }

    pub fn with_data_context(
        mut self,
        branch: &str,
        staged_files: &str,
        recent_commits: &str,
    ) -> Self {
        self.sections
            .push(DataContextSection::new(branch, staged_files, recent_commits).render());
        self
    }

    #[must_use]
    pub fn build(self) -> String {
        self.sections.join("\n")
    }
}
