use cause::{Cause, cause};
use std::process::Command;

use crate::common::ErrorType;
use crate::models::repo_config::RepositoryConfiguration;

pub struct RepositoryFetcher;

impl RepositoryFetcher {
    /// Fetch a repository to a temporary directory
    /// This is a wrapper around the existing fetch functionality with caching logic
    pub fn fetch_repository(
        &self,
        config: &RepositoryConfiguration,
        cache_path: &str,
    ) -> Result<(), Cause<ErrorType>> {
        // Check if the repository is already cached and up-to-date
        if self.is_cache_valid(config, cache_path)? {
            println!("Using cached repository: {}", config.url);
            return Ok(());
        }

        println!("Fetching repository: {} to cache", config.url);

        self.execute_git_clone(&config.url, cache_path)?;
        self.execute_git_checkout(cache_path, &config.branch)?;

        println!("Repository fetched and cached at: {cache_path}");
        Ok(())
    }

    /// Execute the git clone command with error handling
    fn execute_git_clone(&self, url: &str, cache_path: &str) -> Result<(), Cause<ErrorType>> {
        let args = [
            "clone",
            "--depth",
            "1",
            "--filter=blob:none",
            "--no-checkout",
            "--progress",
            url,
            cache_path,
        ];

        let output = Command::new("git")
            .args(args)
            .output()
            .map_err(|e| cause!(ErrorType::GitCloneCommand).src(e))?;

        if !output.status.success() {
            let error = String::from_utf8(output.stderr)
                .unwrap_or("Could not get error output of git clone command".into());
            return Err(cause!(ErrorType::GitCloneCommandExitStatus, error));
        }

        Ok(())
    }

    /// Execute the git checkout command with error handling
    fn execute_git_checkout(&self, cache_path: &str, branch: &str) -> Result<(), Cause<ErrorType>> {
        let args = ["-C", cache_path, "checkout", branch];

        let output = Command::new("git")
            .args(args)
            .output()
            .map_err(|e| cause!(ErrorType::GitCheckoutCommand).src(e))?;

        if !output.status.success() {
            let error = String::from_utf8(output.stderr)
                .unwrap_or("Could not get error output of git checkout command".into());
            return Err(cause!(ErrorType::GitCheckoutCommandExitStatus, error));
        }

        Ok(())
    }

    /// Check if the cached repository is still valid (up-to-date)
    fn is_cache_valid(
        &self,
        _config: &RepositoryConfiguration,
        cache_path: &str,
    ) -> Result<bool, Cause<ErrorType>> {
        // For now, just check if the directory exists
        // In a real implementation, we would check if the remote has updates
        Ok(std::path::Path::new(cache_path).exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::repo_config::RepositoryConfiguration;

    #[test]
    fn test_fetcher_creation() {
        let _fetcher = RepositoryFetcher;
        // Just checking the struct can be instantiated
    }

    #[test]
    fn test_is_cache_valid_with_nonexistent_path() {
        let _fetcher = RepositoryFetcher;
        let _config = RepositoryConfiguration::new(
            "https://github.com/example/repo.git".to_string(),
            "main".to_string(),
            "./src/module1".to_string(),
            vec!["src/".to_string()],
            None,
        );

        // Test with a path that doesn't exist
        let result = _fetcher.is_cache_valid(&_config, "/definitely/does/not/exist");
        // It should return Ok(false) since the path doesn't exist
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}
