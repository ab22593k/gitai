//! Prompt engineering framework.
//!
//! Provides composable prompt building through the `PromptBuilder` API
//! and the `PromptSection` trait for custom sections.

pub mod builder;
pub mod sections;

#[cfg(test)]
mod tests;
