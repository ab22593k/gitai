use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::repo_config::RepositoryConfiguration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireOperation {
    /// The configuration defining the source
    pub source_config: RepositoryConfiguration,
    /// Path to the cached repository to use
    pub cached_repo_path: String,
    /// Unique identifier for this operation
    pub operation_id: Uuid,
}

impl WireOperation {
    pub fn new(source_config: RepositoryConfiguration, cached_repo_path: String) -> Self {
        Self {
            source_config,
            cached_repo_path,
            operation_id: Uuid::new_v4(),
        }
    }
}
