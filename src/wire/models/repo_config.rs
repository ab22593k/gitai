use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfiguration {
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
}

impl RepositoryConfiguration {
    pub fn new(
        url: String,
        branch: String,
        target_path: String,
        filters: Vec<String>,
        commit_hash: Option<String>,
    ) -> Self {
        Self {
            url,
            branch,
            target_path,
            filters,
            commit_hash,
        }
    }
}
