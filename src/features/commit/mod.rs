#[allow(clippy::uninlined_format_args)]
mod cli;
#[allow(clippy::uninlined_format_args)]
pub mod completion;
mod prompt_optimizer;
mod relevance;
pub mod types;

#[allow(clippy::uninlined_format_args)]
pub mod prompt;
#[allow(clippy::uninlined_format_args)]
pub mod service;

pub use cli::{handle_completion_command, handle_message_command, handle_pr_command};
use git2::FileMode;
pub use service::CommitService;
pub use types::{
    GeneratedMessage, GeneratedPullRequest, format_commit_message, format_pull_request,
};

use crate::git::CommitResult;
use std::fmt::Write;

pub fn format_commit_result(result: &CommitResult, message: &str) -> String {
    let mut output = format!(
        "[{} {}] {}\n",
        result.branch,
        result.commit_hash,
        message.lines().next().unwrap_or("")
    );

    writeln!(
        &mut output,
        " {} file{} changed, {} insertion{}(+), {} deletion{}(-)",
        result.files_changed,
        if result.files_changed == 1 { "" } else { "s" },
        result.insertions,
        if result.insertions == 1 { "" } else { "s" },
        result.deletions,
        if result.deletions == 1 { "" } else { "s" }
    )
    .expect("writing to string should never fail");

    for (file, mode) in &result.new_files {
        writeln!(
            &mut output,
            " create mode {} {}",
            format_file_mode(*mode),
            file
        )
        .expect("writing to string should never fail");
    }

    output
}

fn format_file_mode(mode: FileMode) -> String {
    match mode {
        FileMode::Blob => "100644",
        FileMode::BlobExecutable => "100755",
        FileMode::Link => "120000",
        FileMode::Commit => "160000",
        FileMode::Tree => "040000",
        _ => "000000",
    }
    .to_string()
}
