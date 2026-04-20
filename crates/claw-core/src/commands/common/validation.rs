use crate::llm::context::CommitContext;
use crate::output;
use anyhow::{Result, anyhow};

pub fn validate_staged_files(git_info: &CommitContext) {
    if git_info.staged_files.is_empty() {
        output::print_warning(
            "No staged changes. Please stage your changes before generating a commit message.",
        );
        output::print_info("You can stage changes using 'git add <file>' or 'git add .'");
    }
}

pub fn validate_context_ratio(context_ratio: f32) -> Result<()> {
    if !(0.0..=1.0).contains(&context_ratio) {
        return Err(anyhow!("Context ratio must be between 0.0 and 1.0"));
    }
    Ok(())
}

pub fn validate_environment() -> Result<()> {
    crate::config::Config::load().map_err(|e| anyhow!("Failed to load config: {e}"))?;
    Ok(())
}
