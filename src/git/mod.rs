// Git module providing functionality for Git repository operations

#[allow(clippy::uninlined_format_args)]
mod commit;
#[allow(clippy::uninlined_format_args)]
mod files;
mod history;
mod hooks;
#[allow(clippy::uninlined_format_args)]
mod repository;
mod utils;

// Re-export primary types for public use
pub use commit::CommitInfo;
pub use commit::CommitResult;
pub use repository::GhostRefManager;
pub use repository::GitRepo;

// Re-export utility functions
pub use utils::*;

// Re-export type aliases to maintain backward compatibility
pub use crate::core::context::{RecentCommit, StagedFile};
pub use files::RepoFilesInfo;
