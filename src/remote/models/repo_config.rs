use serde::{Deserialize, Serialize};

use crate::remote::common::{MergeStrategy, Method};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfiguration {
    /// The name of the wire entry
    pub name_filter: Option<String>,
    /// The URL of the remote repository
    pub url: String,
    /// The branch to pull from (default: main/master)
    pub branch: String,
    /// The local path where content should be placed
    pub target_path: String,
    /// Paths/filenames to include from the repository
    pub filters: Vec<String>,
    /// Specific commit to check out (optional)
    pub commit_hash: Option<String>,
    /// Method for cloning
    pub mtd: Option<Method>,
    /// The last synchronized commit hash
    pub last_sync_hash: Option<String>,
    /// Strategy for merging local changes
    pub merge_strategy: Option<MergeStrategy>,
}

impl RepositoryConfiguration {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name_filter: Option<String>,
        url: String,
        branch: String,
        target_path: String,
        filters: Vec<String>,
        commit_hash: Option<String>,
        mtd: Option<Method>,
        last_sync_hash: Option<String>,
        merge_strategy: Option<MergeStrategy>,
    ) -> Self {
        Self {
            name_filter,
            url,
            branch,
            target_path,
            filters,
            commit_hash,
            mtd,
            last_sync_hash,
            merge_strategy,
        }
    }
}
