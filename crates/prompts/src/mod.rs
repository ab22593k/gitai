//! Prompt engineering framework + domain-specific prompt templates.
//!
//! The `builder` and `sections` modules provide a composable prompt-building API.
//! The `commit`, `changelog`, `pr`, and `notes` modules provide ready-to-use
//! prompt template functions for each domain.

pub mod builder;
pub mod changelog;
pub mod commit;
pub mod notes;
pub mod pr;
pub mod sections;
