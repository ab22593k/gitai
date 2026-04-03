#![allow(clippy::duplicate_mod)]

// Test module declarations
mod common;

mod git;
mod llm;
mod sync;

// Root level test files
mod binary_detection_tests;
mod common_params_tests;
mod concurrent_access_tests;
mod config_tests;
mod end_to_end_tests;
mod error_quality_tests;
mod feature_intent_tests;
mod regression_tests;
mod service_tests;
mod user_experience_tests;
