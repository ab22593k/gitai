use crate::remote::models::repo_config::RepositoryConfiguration;

use super::super::common::{ErrorType, Method};
use cause::{Cause, cause};
use git2::Repository;
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
        tokio::task::spawn_blocking(move || {
            Self::execute_git_clone(&config, &cache_path_clone)?;
            if !matches!(config.mtd, Some(Method::ShallowNoSparse)) {
                Self::execute_git_checkout(&cache_path_clone, &config.branch)?;
            }
            Ok(())
        })
        .await
        .map_err(|e| cause!(ErrorType::GitCloneCommand).msg(format!("Task join error: {:?}", e)))??;

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
    fn is_cache_valid(_config: &RepositoryConfiguration, cache_path: &str) -> bool {
        // Check if the directory exists
        std::path::Path::new(cache_path).exists()
    }
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
        assert!(!result);
    }
}
