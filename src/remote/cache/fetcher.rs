use crate::remote::models::repo_config::RepositoryConfiguration;

use super::super::common::{ErrorType, Method};
use cause::{Cause, cause};
use git2::Repository;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Clone)]
pub struct RepositoryFetcher;

impl RepositoryFetcher {
    /// Fetch a repository to a temporary directory
    /// This is a wrapper around the existing fetch functionality with caching logic
    pub async fn fetch_repository(
        &self,
        config: &RepositoryConfiguration,
        cache_path: &str,
    ) -> Result<(), Cause<ErrorType>> {
        let config = config.clone();
        let cache_path = cache_path.to_string();

        // Check if the repository is already cached and up-to-date
        if Self::is_cache_valid(&config, &cache_path) {
            println!("Using cached repository: {}", config.url);
            return Ok(());
        }

        println!("Fetching repository: {} to cache", config.url);

        // Wrap blocking operations in spawn_blocking
        let cache_path_clone = cache_path.clone();
        let config_clone = config.clone();
        tokio::task::spawn_blocking(move || -> Result<(), Cause<ErrorType>> {
            Self::execute_git_clone(&config_clone, &cache_path_clone)?;
            if !matches!(config_clone.mtd, Some(Method::ShallowNoSparse)) {
                Self::execute_git_checkout(&cache_path_clone, &config_clone.branch)?;
            }

            // Get the actual commit hash and write cache metadata
            let commit_hash = Self::get_current_commit_hash(&cache_path_clone)?;
            Self::write_cache_metadata(&config_clone, &cache_path_clone, &commit_hash)?;

            Ok(())
        })
        .await
        .map_err(|e| cause!(ErrorType::GitCloneCommand).msg(format!("Task join error: {e:?}")))??;

        println!("Repository fetched and cached at: {cache_path}");
        Ok(())
    }

    /// Execute the git clone command with error handling
    fn execute_git_clone(
        config: &RepositoryConfiguration,
        cache_path: &str,
    ) -> Result<(), Cause<ErrorType>> {
        // Remove the cache directory if it exists
        if std::path::Path::new(cache_path).exists() {
            std::fs::remove_dir_all(cache_path)
                .map_err(|e| cause!(ErrorType::GitCloneCommand).src(e))?;
        }

        if matches!(config.mtd, Some(Method::ShallowNoSparse)) {
            // Use git command for shallow clone with branch
            let output = Command::new("git")
                .args([
                    "clone",
                    "--depth",
                    "1",
                    "--branch",
                    &config.branch,
                    &config.url,
                    cache_path,
                ])
                .output()
                .map_err(|e| cause!(ErrorType::GitCloneCommand).src(e))?;
            if !output.status.success() {
                return Err(
                    cause!(ErrorType::GitCloneCommand).msg(String::from_utf8_lossy(&output.stderr))
                );
            }
        } else {
            Repository::clone(&config.url, cache_path)
                .map_err(|e| cause!(ErrorType::GitCloneCommand).src(e))?;
        }

        Ok(())
    }

    /// Execute the git checkout command with error handling
    fn execute_git_checkout(cache_path: &str, rev: &str) -> Result<(), Cause<ErrorType>> {
        let repo = Repository::open(cache_path)
            .map_err(|e| cause!(ErrorType::GitCheckoutCommand).src(e))?;

        let obj = repo
            .revparse_single(rev)
            .map_err(|e| cause!(ErrorType::GitCheckoutCommand).src(e))?
            .peel(git2::ObjectType::Commit)
            .map_err(|e| cause!(ErrorType::GitCheckoutCommand).src(e))?;

        repo.checkout_tree(&obj, None)
            .map_err(|e| cause!(ErrorType::GitCheckoutCommand).src(e))?;

        repo.set_head_detached(obj.id())
            .map_err(|e| cause!(ErrorType::GitCheckoutCommand).src(e))?;

        Ok(())
    }

    /// Check if the cached repository is still valid (up-to-date)
    fn is_cache_valid(config: &RepositoryConfiguration, cache_path: &str) -> bool {
        let path = std::path::Path::new(cache_path);
        if !path.exists() {
            return false;
        }

        // Check for .git directory to verify it's a valid git repo
        if !path.join(".git").exists() {
            return false;
        }

        // Try to read the cached commit hash if available
        let metadata_path = path.join(".cache_metadata");
        if let Ok(metadata) = std::fs::read_to_string(metadata_path)
            && let Ok(cached) = serde_json::from_str::<CacheEntry>(&metadata)
        {
            // Verify URL matches
            if cached.url != config.url {
                return false;
            }
            // Verify branch matches
            if cached.branch != config.branch {
                return false;
            }
            // If config specifies a commit hash, verify it matches
            if let Some(ref requested_commit) = config.commit_hash
                && cached.commit_hash != *requested_commit
            {
                return false;
            }
            // Cache is valid
            return true;
        }

        // No metadata found, but directory exists - assume stale
        false
    }

    /// Get the current HEAD commit hash from a repository
    fn get_current_commit_hash(cache_path: &str) -> Result<String, Cause<ErrorType>> {
        let repo = git2::Repository::open(cache_path)
            .map_err(|e| cause!(ErrorType::GitCheckoutCommand).src(e))?;

        let head = repo
            .head()
            .map_err(|e| cause!(ErrorType::GitCheckoutCommand).src(e))?;

        let oid = head
            .target()
            .ok_or_else(|| cause!(ErrorType::GitCheckoutCommand, "HEAD is not a commit"))?;

        Ok(oid.to_string())
    }

    /// Write cache metadata after successful fetch
    fn write_cache_metadata(
        config: &RepositoryConfiguration,
        cache_path: &str,
        commit_hash: &str,
    ) -> Result<(), Cause<ErrorType>> {
        let metadata = CacheEntry {
            url: config.url.clone(),
            branch: config.branch.clone(),
            commit_hash: commit_hash.to_string(),
            cached_at: chrono::Utc::now().to_rfc3339(),
        };

        let metadata_path = std::path::Path::new(cache_path).join(".cache_metadata");
        let json = serde_json::to_string(&metadata).map_err(|e| {
            cause!(ErrorType::GitCloneCommand)
                .msg(format!("Failed to serialize cache metadata: {e}"))
        })?;

        std::fs::write(&metadata_path, json)
            .map_err(|e| cause!(ErrorType::GitCloneCommand).src(e))?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    url: String,
    branch: String,
    commit_hash: String,
    cached_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetcher_creation() {
        let _ = RepositoryFetcher;
        // Just checking the struct can be instantiated
    }

    #[test]
    fn test_is_cache_valid_with_nonexistent_path() {
        let config = RepositoryConfiguration::new(
            "https://github.com/example/repo.git".to_string(),
            "main".to_string(),
            "./src/module1".to_string(),
            vec!["src/".to_string()],
            None,
            None,
        );

        // Test with a path that doesn't exist
        let result = RepositoryFetcher::is_cache_valid(&config, "/definitely/does/not/exist");
        // It should return false since the path doesn't exist
        assert!(
            !result,
            "Expected cache to be invalid for non-existent path"
        );
    }
}
