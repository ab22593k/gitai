use gitai::{
    CacheManager, RepositoryConfiguration, remote::cache::key_generator::CacheKeyGenerator,
};
use std::time::Instant;

#[test]
fn test_end_to_end_caching_scenario() {
    let start_time = Instant::now();

    // Simulate the scenario from quickstart.md:
    // Create a configuration with multiple entries for the same repository
    let configs = vec![
        RepositoryConfiguration::new(
            "https://github.com/example/repo.git".to_string(),
            "main".to_string(),
            "./src/module1".to_string(),
            vec!["src/".to_string(), "lib/".to_string()],
            None,
            None,
        ),
        RepositoryConfiguration::new(
            "https://github.com/example/repo.git".to_string(), // Same repo
            "main".to_string(),
            "./src/module2".to_string(),
            vec!["utils/".to_string()],
            None,
            None,
        ),
    ];

    // Create a cache manager
    let cache_manager = CacheManager::new();

    // Plan fetch operations - this should identify that there's only 1 unique repository
    let (unique_configs, wire_operations) = cache_manager
        .plan_fetch_operations(&configs)
        .expect("Failed to plan fetch operations");

    // Verify that we have correctly identified only 1 unique repository instead of 2
    assert_eq!(
        unique_configs.len(),
        1,
        "Should have identified only 1 unique repository"
    );
    assert_eq!(
        wire_operations.len(),
        2,
        "Should have created 2 wire operations"
    );

    // Simulate generating cache keys
    let cache_key = CacheKeyGenerator::generate_key(&unique_configs[0]);
    assert!(!cache_key.is_empty(), "Cache key should be generated");

    // The test validates that:
    // 1. Duplicate repositories are identified (only 1 unique instead of 2)
    // 2. Proper wire operations are created for each original config
    // 3. Cache keys are properly generated

    println!(
        "End-to-end test completed successfully in {:?}",
        start_time.elapsed()
    );
    println!("Input configurations: {}", configs.len());
    println!("Unique repositories to fetch: {}", unique_configs.len());
    println!("Wire operations generated: {}", wire_operations.len());

    // This demonstrates the core functionality: avoiding multiple pulls of the same repository
    assert!(
        configs.len() > unique_configs.len(),
        "Should have reduced multiple configs to fewer unique repositories"
    );
}

#[test]
fn test_cache_performance_improvement_simulation() {
    // Simulate the performance improvement by comparing operations needed
    let start_time = Instant::now();

    // Create many configurations that reference the same repository
    let mut configs = Vec::new();
    for i in 0..10 {
        configs.push(RepositoryConfiguration::new(
            "https://github.com/example/repo.git".to_string(), // Same repo
            "main".to_string(),
            format!("./src/module{i}"),
            vec![format!("src{i}")],
            None,
            None,
        ));
    }

    // Create a cache manager
    let cache_manager = CacheManager::new();

    // Plan fetch operations
    let (unique_configs, wire_operations) = cache_manager
        .plan_fetch_operations(&configs)
        .expect("Failed to plan fetch operations");

    let elapsed = start_time.elapsed();

    // Verify the performance improvement
    assert_eq!(
        unique_configs.len(),
        1,
        "Should only need to fetch 1 unique repository"
    );
    assert_eq!(
        wire_operations.len(),
        10,
        "Should have 10 wire operations for different targets"
    );

    println!("Performance test completed in {elapsed:?}");
    println!(
        "Would have required {} separate git pulls without caching, now only requires 1",
        configs.len()
    );

    // Assert that we achieved the expected performance improvement (simulated)
    assert!(
        configs.len() > unique_configs.len(),
        "Caching should reduce the number of required git operations"
    );
}
