use gitai::{CachedRepository, RepositoryConfiguration, WireOperation};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

#[test]
fn test_concurrent_access_to_same_repository() {
    const NUM_THREADS: usize = 5;

    // Create a shared cached repository wrapped in Arc<Mutex<>> to simulate serialized access
    let cached_repo = Arc::new(Mutex::new(CachedRepository::new(
        "https://github.com/example/repo.git".to_string(),
        "main".to_string(),
        "/tmp/cache/repo".to_string(),
        "abc123".to_string(),
    )));

    // Spawn multiple threads to simulate concurrent access to the shared repository
    let mut handles = Vec::new();

    for i in 0..NUM_THREADS {
        let repo_clone = Arc::clone(&cached_repo);
        let thread_id = i;

        let handle = thread::spawn(move || {
            // Acquire the lock on the shared repository (this serializes access)
            let repo = repo_clone
                .lock()
                .expect("Failed to acquire repository lock");

            // Create a configuration specific to this thread
            let config = RepositoryConfiguration {
                url: repo.url.clone(),
                branch: repo.branch.clone(),
                target_path: format!("./src/module{thread_id}"),
                filters: vec![format!("src{thread_id}")],
                commit_hash: None,
                mtd: None,
            };

            // Create a wire operation (unused but simulates the operation creation)
            let _op = WireOperation::new(config, repo.local_cache_path.clone());

            // Simulate some work with a short delay
            thread::sleep(Duration::from_millis(10));

            // Return a completion message
            format!("Operation {thread_id} completed")
        });

        handles.push(handle);
    }

    // Wait for all threads to complete and collect their results
    let results: Vec<String> = handles
        .into_iter()
        .map(|h| h.join().expect("Thread panicked during execution"))
        .collect();

    // Verify that all threads completed successfully
    assert_eq!(results.len(), NUM_THREADS);

    for (i, result) in results.iter().enumerate() {
        assert_eq!(result.as_str(), format!("Operation {i} completed"));
    }
}

#[test]
fn test_concurrent_repository_access_simulation() {
    const NUM_THREADS: usize = 3;

    // Simulate a repository lock using AtomicBool for atomic operations
    let repo_in_use = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();

    // Spawn multiple threads attempting to acquire access to the shared resource
    for i in 0..NUM_THREADS {
        let repo_in_use_clone = Arc::clone(&repo_in_use);
        let thread_id = i;

        let handle = thread::spawn(move || {
            // Attempt to atomically acquire the lock (only one thread succeeds)
            let acquired = repo_in_use_clone.compare_exchange(
                false, // Expected: lock is free
                true,  // Set to: lock is taken
                Ordering::SeqCst,
                Ordering::SeqCst,
            );

            match acquired {
                Ok(_) => {
                    // Successfully acquired the lock; simulate work
                    thread::sleep(Duration::from_millis(50));

                    // Release the lock
                    repo_in_use_clone.store(false, Ordering::SeqCst);

                    format!("Thread {thread_id} acquired and released repo")
                }
                Err(_) => {
                    // Failed to acquire; in a real scenario, would wait or handle differently
                    format!("Thread {thread_id} could not acquire repo")
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all threads to complete and collect results
    let results: Vec<String> = handles
        .into_iter()
        .map(|h| h.join().expect("Thread panicked during execution"))
        .collect();

    // Ensure at least one thread successfully acquired and released the lock
    assert!(results.iter().any(|r| r.contains("acquired and released")));
}
