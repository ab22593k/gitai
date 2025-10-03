use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::wire::models::repo_config::RepositoryConfiguration;

// Type alias for cache key
type CacheKey = String;

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime::now() should always be after UNIX_EPOCH")
        .as_secs()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    /// The repository URL
    pub repo_url: String,
    /// The branch that was cached
    pub branch: String,
    /// The commit hash at the time of caching
    pub commit_hash: String,
    /// When this cache entry was created
    pub created_at: u64,
    /// When this cache entry was last accessed
    pub last_accessed: u64,
    /// Size of the cached repository in bytes
    pub size_bytes: u64,
    /// The path to the cached repository
    pub cache_path: String,
}

impl CacheMetadata {
    pub fn new(config: &RepositoryConfiguration, cache_path: &str, commit_hash: &str) -> Self {
        let now = current_timestamp();

        // Get the directory size
        let size = get_directory_size(cache_path);

        Self {
            repo_url: config.url.clone(),
            branch: config.branch.clone(),
            commit_hash: commit_hash.to_string(),
            created_at: now,
            last_accessed: now,
            size_bytes: size,
            cache_path: cache_path.to_string(),
        }
    }

    /// Update the last accessed time
    pub fn update_access_time(&mut self) {
        self.last_accessed = current_timestamp();
    }
}

pub struct CacheMetadataManager {
    /// In-memory cache of metadata
    metadata: HashMap<CacheKey, CacheMetadata>,
    /// Path to store metadata on disk
    metadata_file_path: String,
}

impl CacheMetadataManager {
    pub fn new(metadata_file_path: String) -> Self {
        let mut manager = Self {
            metadata: HashMap::new(),
            metadata_file_path,
        };

        // Load existing metadata from file
        manager.load_from_disk().ok();
        manager
    }

    /// Store metadata for a cache entry
    pub fn store_metadata(&mut self, key: &str, metadata: CacheMetadata) -> Result<(), String> {
        self.metadata.insert(key.to_string(), metadata);
        self.save_to_disk()
    }

    /// Retrieve metadata for a cache key
    pub fn get_metadata(&self, key: &str) -> Option<&CacheMetadata> {
        self.metadata.get(key)
    }

    /// Update the access time for a cache entry
    pub fn update_access_time(&mut self, key: &str) -> Result<(), String> {
        if let Some(metadata) = self.metadata.get_mut(key) {
            metadata.update_access_time();
            self.save_to_disk()
        } else {
            Err(format!("Metadata not found for key: {key}"))
        }
    }

    /// Check if a cache entry exists and is still valid
    pub fn is_cache_valid(&self, key: &str) -> bool {
        self.metadata.contains_key(key)
    }

    /// Remove metadata for a cache key
    pub fn remove_metadata(&mut self, key: &str) -> Result<(), String> {
        self.metadata.remove(key);
        self.save_to_disk()
    }

    /// Get all cache keys
    pub fn get_all_keys(&self) -> Vec<String> {
        self.metadata.keys().cloned().collect()
    }

    /// Save metadata to disk
    fn save_to_disk(&self) -> Result<(), String> {
        // Create directory if it doesn't exist
        if let Some(parent) = Path::new(&self.metadata_file_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create metadata directory: {e}"))?;
        }

        let json = serde_json::to_string_pretty(&self.metadata)
            .map_err(|e| format!("Failed to serialize metadata: {e}"))?;

        fs::write(&self.metadata_file_path, json)
            .map_err(|e| format!("Failed to write metadata file: {e}"))?;

        Ok(())
    }

    /// Load metadata from disk
    fn load_from_disk(&mut self) -> Result<(), String> {
        if !Path::new(&self.metadata_file_path).exists() {
            // File doesn't exist yet, that's OK
            return Ok(());
        }

        let json = fs::read_to_string(&self.metadata_file_path)
            .map_err(|e| format!("Failed to read metadata file: {e}"))?;

        self.metadata = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to deserialize metadata: {e}"))?;

        Ok(())
    }

    /// Clean up old cache entries based on last access time
    pub fn cleanup_old_entries(&mut self, max_age_seconds: u64) -> Result<Vec<String>, String> {
        let now = current_timestamp();

        let mut to_remove = Vec::new();
        for (key, metadata) in &self.metadata {
            if now - metadata.last_accessed > max_age_seconds {
                to_remove.push(key.clone());
            }
        }

        // Remove the old entries
        for key in &to_remove {
            self.metadata.remove(key);
            // Note: We're not actually deleting the cache directory here,
            // that would be handled separately
        }

        if !to_remove.is_empty() {
            self.save_to_disk()?;
        }

        Ok(to_remove)
    }
}

/// Helper function to get directory size (simplified implementation)
fn get_directory_size(path: &str) -> u64 {
    let mut size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries {
            if let Ok(entry) = entry
                && let Ok(metadata) = entry.metadata()
            {
                if metadata.is_file() {
                    size += metadata.len();
                } else if metadata.is_dir() {
                    // For simplicity, we're not recursively calculating subdirectory sizes
                    size += 1024; // Estimate 1KB for directory
                }
            }
        }
    }
    size
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_metadata_creation() {
        let config = RepositoryConfiguration::new(
            "https://github.com/example/repo.git".to_string(),
            "main".to_string(),
            "./src/module1".to_string(),
            vec!["src/".to_string()],
            None,
        );

        let temp_dir = TempDir::new().expect("Failed to create temporary directory for test");
        let cache_path = temp_dir
            .path()
            .to_str()
            .expect("Failed to convert temporary directory path to string");

        let metadata = CacheMetadata::new(&config, cache_path, "abc123");

        assert_eq!(metadata.repo_url, "https://github.com/example/repo.git");
        assert_eq!(metadata.branch, "main");
        assert_eq!(metadata.commit_hash, "abc123");
        assert_eq!(metadata.cache_path, cache_path);
    }

    #[test]
    fn test_cache_metadata_manager() {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory for test");
        let metadata_file = temp_dir
            .path()
            .join("metadata.json")
            .to_str()
            .expect("Failed to convert path to string")
            .to_string();

        let mut manager = CacheMetadataManager::new(metadata_file);

        // Create test metadata
        let config = RepositoryConfiguration::new(
            "https://github.com/example/repo.git".to_string(),
            "main".to_string(),
            "./src/module1".to_string(),
            vec!["src/".to_string()],
            None,
        );

        let cache_path_binding = temp_dir.path().join("cache");
        let test_cache_path = cache_path_binding
            .to_str()
            .expect("Failed to convert cache path to string");

        let metadata = CacheMetadata::new(&config, test_cache_path, "abc123");
        let key = "test-key";

        // Store metadata
        manager
            .store_metadata(key, metadata.clone())
            .expect("Failed to store metadata");

        // Retrieve metadata
        let retrieved = manager
            .get_metadata(key)
            .expect("Failed to retrieve stored metadata");
        assert_eq!(retrieved.repo_url, metadata.repo_url);

        // Update access time
        manager
            .update_access_time(key)
            .expect("Failed to update access time");

        // Check if cache is valid
        assert!(manager.is_cache_valid(key));

        // Get all keys
        let keys = manager.get_all_keys();
        assert_eq!(keys, vec!["test-key".to_string()]);
    }
}
