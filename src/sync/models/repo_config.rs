use serde::{Deserialize, Serialize};

use crate::sync::common::{MergeStrategy, Method};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfiguration {
    pub name_filter: Option<String>,
    pub url: String,
    pub branch: String,
    pub target_path: String,
    pub filters: Vec<String>,
    pub commit_hash: Option<String>,
    pub mtd: Option<Method>,
    pub last_sync_hash: Option<String>,
    pub merge_strategy: Option<MergeStrategy>,
}

impl Default for RepositoryConfiguration {
    fn default() -> Self {
        Self {
            name_filter: None,
            url: String::new(),
            branch: String::from("main"),
            target_path: String::new(),
            filters: Vec::new(),
            commit_hash: None,
            mtd: None,
            last_sync_hash: None,
            merge_strategy: None,
        }
    }
}
