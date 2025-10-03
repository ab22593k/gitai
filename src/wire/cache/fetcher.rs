use cause::{Cause, cause};
use git2::Repository;

use super::super::common::ErrorType;
use crate::wire::models::repo_config::RepositoryConfiguration;

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
        if Self::is_cache_valid(config, cache_path) {
            println!("Using cached repository: {}", config.url);
            return Ok(());
        }

        println!("Fetching repository: {} to cache", config.url);

        Self::execute_git_clone(&config.url, cache_path)?;
        Self::execute_git_checkout(cache_path, &config.branch)?;

        println!("Repository fetched and cached at: {cache_path}");
        Ok(())
    }

    /// Execute the git clone command with error handling
    fn execute_git_clone(url: &str, cache_path: &str) -> Result<(), Cause<ErrorType>> {
        Repository::clone(url, cache_path)
            .map_err(|e| cause!(ErrorType::GitCloneCommand).src(e))?;

        Ok(())
    }

    /// Execute the git checkout command with error handling
    fn execute_git_checkout(cache_path: &str, branch: &str) -> Result<(), Cause<ErrorType>> {
        let repo = Repository::open(cache_path)
            .map_err(|e| cause!(ErrorType::GitCheckoutCommand).src(e))?;

        let branch_ref = repo
            .find_branch(branch, git2::BranchType::Local)
            .or_else(|_| repo.find_branch(branch, git2::BranchType::Remote))
            .map_err(|e| cause!(ErrorType::GitCheckoutCommand).src(e))?;

        let obj = branch_ref
            .get()
            .peel(git2::ObjectType::Commit)
            .map_err(|e| cause!(ErrorType::GitCheckoutCommand).src(e))?;

        repo.checkout_tree(&obj, None)
            .map_err(|e| cause!(ErrorType::GitCheckoutCommand).src(e))?;

        repo.set_head(&format!("refs/heads/{branch}"))
            .map_err(|e| cause!(ErrorType::GitCheckoutCommand).src(e))?;

        Ok(())
    }

    /// Check if the cached repository is still valid (up-to-date)
    fn is_cache_valid(_config: &RepositoryConfiguration, cache_path: &str) -> bool {
        // For now, just check if the directory exists
        // In a real implementation, we would check if the remote has updates
        std::path::Path::new(cache_path).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::models::repo_config::RepositoryConfiguration;

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
        );

        // Test with a path that doesn't exist
        let result = RepositoryFetcher::is_cache_valid(&config, "/definitely/does/not/exist");
        // It should return false since the path doesn't exist
        assert!(!result);
    }
}
