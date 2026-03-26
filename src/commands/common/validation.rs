use crate::llm::context::CommitContext;
use crate::ui;

pub fn validate_staged_files(git_info: &CommitContext, dry: bool) {
    if git_info.staged_files.is_empty() && !dry {
        ui::print_warning(
            "No staged changes. Please stage your changes before generating a commit message.",
        );
        ui::print_info("You can stage changes using 'git add <file>' or 'git add .'");
    }
}

pub fn validate_context_ratio(context_ratio: f32) -> Result<(), String> {
    if !(0.0..=1.0).contains(&context_ratio) {
        return Err("Context ratio must be between 0.0 and 1.0".to_string());
    }
    Ok(())
}

pub fn validate_environment() -> Result<(), String> {
    if let Err(e) = crate::config::Config::load() {
        return Err(format!("Failed to load config: {e}"));
    }
    Ok(())
}
