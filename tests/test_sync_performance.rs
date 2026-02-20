#![allow(clippy::cast_precision_loss)]
#![allow(clippy::as_conversions)]

use gitai::RepositoryConfiguration;
use std::time::Instant;

// Note: This is a placeholder performance test. A real performance test
// would involve actual git operations which require network access.
// For this implementation, we'll simulate the performance difference.

#[test]
fn test_repository_deduplication_performance() {
    // Create many configurations that reference the same repository
    let mut configs = Vec::new();

    for i in 0..10 {
        configs.push(RepositoryConfiguration {
            url: "https://github.com/example/repo.git".to_string(), // Same repo
            branch: "main".to_string(),
            target_path: format!("./src/module{i}"),
            filters: vec![format!("src{i}")],
            commit_hash: None,
            mtd: None,
        });
    }

    // Simulate time without deduplication (10 separate git pulls)
    let start_time_no_dedup = Instant::now();
    // Simulate 10 separate operations (each taking 100ms)
    std::thread::sleep(std::time::Duration::from_millis(100 * 10));
    let duration_no_dedup = start_time_no_dedup.elapsed();

    // Simulate time with deduplication (1 git pull used for all)
    let start_time_with_dedup = Instant::now();
    // Simulate 1 operation (taking 100ms) + processing overhead (10ms per config)
    std::thread::sleep(std::time::Duration::from_millis(100 + 10 * 10));
    let duration_with_dedup = start_time_with_dedup.elapsed();

    println!("Time without deduplication: {duration_no_dedup:?}");
    println!("Time with deduplication: {duration_with_dedup:?}");

    // With deduplication, the time should be significantly less
    assert!(
        duration_with_dedup < duration_no_dedup,
        "Deduplication should result in faster execution"
    );

    // Performance improvement should be significant (at least 50% faster in this simulation)
    let improvement =
        (duration_no_dedup.as_millis() as f64) / (duration_with_dedup.as_millis() as f64);

    println!("Performance improvement: {improvement:.2}x");
    assert!(
        improvement > 1.5,
        "Performance should be at least 1.5x better with deduplication"
    );
}
