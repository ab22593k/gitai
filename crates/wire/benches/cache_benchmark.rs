// Performance benchmarks for git-wire caching functionality
// Note: This is a basic benchmark structure - actual benchmarks would require more setup

use gwtwire::{CacheManager, RepositoryConfiguration};
use std::time::Instant;

fn benchmark_cache_operations() {
    // Create a cache manager
    let cache_manager = CacheManager::new();

    // Create multiple repository configurations (some duplicates to test deduplication)
    let configs = vec![
        RepositoryConfiguration::new(
            "https://github.com/example/repo1.git".to_string(),
            "main".to_string(),
            "./src/module1".to_string(),
            vec!["src/".to_string()],
            None,
        ),
        RepositoryConfiguration::new(
            "https://github.com/example/repo1.git".to_string(), // Same repo
            "main".to_string(),
            "./src/module2".to_string(),
            vec!["lib/".to_string()],
            None,
        ),
        RepositoryConfiguration::new(
            "https://github.com/example/repo2.git".to_string(), // Different repo
            "main".to_string(),
            "./src/module3".to_string(),
            vec!["utils/".to_string()],
            None,
        ),
    ];

    // Benchmark the planning operation
    let start = Instant::now();
    let _result = cache_manager.plan_fetch_operations(&configs).unwrap();
    let duration = start.elapsed();

    println!("Cache planning operation took: {duration:?}");
    println!(
        "Processed {} configurations into unique operations",
        configs.len()
    );
}

fn benchmark_repository_deduplication() {
    // Create a cache manager
    let cache_manager = CacheManager::new();

    // Create many configurations with duplicate repositories
    let mut configs = Vec::new();
    for i in 0..100 {
        configs.push(RepositoryConfiguration::new(
            format!("https://github.com/example/repo{}.git", i % 10), // Only 10 unique repos
            "main".to_string(),
            format!("./src/module{i}"),
            vec!["src/".to_string()],
            None,
        ));
    }

    // Benchmark the planning operation
    let start = Instant::now();
    let (unique_configs, operations) = cache_manager.plan_fetch_operations(&configs).unwrap();
    let duration = start.elapsed();

    println!("Deduplication benchmark:");
    println!("  - Input: {} repository configurations", configs.len());
    println!("  - Unique: {} repositories to fetch", unique_configs.len());
    println!("  - Operations: {} wire operations", operations.len());
    println!("  - Time taken: {duration:?}");
}

// TODO: For actual benchmarking, we would use the criterion crate
// but for this implementation, we'll just run the functions in tests
fn main() {
    println!("Running git-wire performance benchmarks...");
    benchmark_cache_operations();
    benchmark_repository_deduplication();
    println!("Benchmarks completed.");
}

#[cfg(test)]
mod bench_tests {
    use super::*;

    #[test]
    fn test_cache_operations_benchmark() {
        benchmark_cache_operations();
    }

    #[test]
    fn test_repository_deduplication_benchmark() {
        benchmark_repository_deduplication();
    }
}
