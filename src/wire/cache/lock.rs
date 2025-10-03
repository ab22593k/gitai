use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Type alias for repository URL
type RepoUrl = String;

#[derive(Default)]
pub struct RepositoryLockManager {
    // Tracks locks for each repository
    locks: Arc<Mutex<HashMap<RepoUrl, Arc<Mutex<bool>>>>>,
}

impl RepositoryLockManager {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Acquire a lock for a specific repository, blocking until available
    pub fn acquire_lock(&self, repo_url: &str) -> Result<(), String> {
        let mut locks = self
            .locks
            .lock()
            .expect("Failed to acquire global lock for repository locks");

        // Check if we already have a lock for this URL
        let repo_lock = locks
            .entry(repo_url.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(false)));

        // Clone the Arc to use for locking
        let lock_clone = Arc::clone(repo_lock);
        drop(locks); // Release the global lock

        // Acquire the specific repository lock
        let guard = lock_clone
            .lock()
            .unwrap_or_else(|_| panic!("Failed to acquire repository lock for URL: {repo_url}"));

        // Hold the lock for the duration of the function, then release it
        // In a real implementation, you'd want to return a guard that manages the lock lifetime
        // For now, just simulate the lock being held briefly
        std::mem::drop(guard);

        Ok(())
    }

    /// Try to acquire a lock for a specific repository without blocking
    pub fn try_acquire_lock(&self, repo_url: &str) -> Result<bool, String> {
        let mut locks = self
            .locks
            .lock()
            .expect("Failed to acquire global lock for repository locks");

        // Check if we already have a lock for this URL
        let repo_lock = locks
            .entry(repo_url.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(false)));

        // Clone the Arc to use for locking
        let lock_clone = Arc::clone(repo_lock);
        drop(locks); // Release the global lock

        // Try to acquire the specific repository lock
        match lock_clone.try_lock() {
            Ok(guard) => {
                // Successfully acquired the lock
                std::mem::drop(guard); // Release immediately for this simplified version
                Ok(true)
            }
            Err(_) => Ok(false), // Lock is already held by another thread
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_manager_creation() {
        let lock_manager = RepositoryLockManager::new();
        assert_eq!(
            lock_manager
                .locks
                .lock()
                .expect("Failed to acquire lock on repository locks map")
                .len(),
            0
        );
    }

    #[test]
    fn test_acquire_lock() {
        let lock_manager = RepositoryLockManager::new();
        let repo_url = "https://github.com/example/repo.git";

        let result = lock_manager.acquire_lock(repo_url);
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_acquire_lock() {
        let lock_manager = RepositoryLockManager::new();
        let repo_url = "https://github.com/example/repo.git";

        // Initially should be able to acquire
        let result = lock_manager.try_acquire_lock(repo_url);
        assert!(result.is_ok());
        assert!(
            result.expect("Failed to unwrap try_acquire_lock result"),
            "Expected to acquire lock successfully on first attempt"
        );
    }
}
