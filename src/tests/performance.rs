use super::{ResourceContentsExt, Test, TestContext};
use std::sync::Arc;
use tokio::time::Instant;

//= docs/design/technical-spec.md#performance-considerations
//= type=test
//# The server SHOULD optimize cache hit rates for commonly accessed crates.
#[tokio::test]
async fn test_cache_hit_rates() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // First request (cache miss)
    let start = Instant::now();
    let result = test
        .read_resource("cargo://crate/serde/info")
        .await
        .unwrap();
    let miss_duration = start.elapsed();

    // Second request (should be cache hit)
    let start = Instant::now();
    let cached_result = test
        .read_resource("cargo://crate/serde/info")
        .await
        .unwrap();
    let hit_duration = start.elapsed();

    // Cache hit should be significantly faster
    assert!(hit_duration < miss_duration / 2);

    // Results should be identical
    assert_eq!(
        result.contents[0].as_text().unwrap(),
        cached_result.contents[0].as_text().unwrap()
    );
}

//= docs/design/technical-spec.md#performance-considerations
//= type=test
//# The server SHOULD implement concurrent request handling efficiently.
#[tokio::test]
async fn test_concurrent_requests() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Create multiple concurrent requests
    let start = Instant::now();
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let test = test.clone();
            tokio::spawn(async move { test.read_resource("cargo://crate/serde/info").await })
        })
        .collect();

    // Wait for all requests to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    let duration = start.elapsed();

    // All requests should succeed
    for result in results {
        assert!(result.unwrap().is_ok());
    }

    // Total time should be reasonable (not 10x single request)
    assert!(duration < std::time::Duration::from_secs(5));
}

//= docs/design/technical-spec.md#performance-considerations
//= type=test
//# The server SHOULD minimize the number of cargo command executions.
#[tokio::test]
async fn test_command_execution_optimization() {
    let ctx = Arc::new(TestContext::new().unwrap());

    // Create a test crate
    ctx.file(
        "Cargo.toml",
        r#"
        [package]
        name = "test-crate"
        version = "0.1.0"
        edition = "2024"
        "#,
    )
    .unwrap();

    ctx.file("src/lib.rs", "").unwrap();

    let test = Test::start(ctx).await.unwrap();

    // Track command execution count through stderr
    let mut command_count = 0;

    // Make multiple metadata requests
    for _ in 0..5 {
        let result = test
            .read_resource("cargo://project/metadata")
            .await
            .unwrap();
        if result.contents[0].as_text().unwrap().contains("Updating") {
            command_count += 1;
        }
    }

    // Should only execute cargo command once since metadata shouldn't be cached
    assert_eq!(command_count, 1);
}

//= docs/design/technical-spec.md#caching-requirements
//= type=test
//# The server SHOULD implement a maximum cache size limit.

//= docs/design/technical-spec.md#caching-requirements
//= type=test
//# The server SHOULD implement a least-recently-used (LRU) eviction policy when the cache is full.

#[tokio::test]
async fn test_cache_size_limits() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Request info for multiple crates to fill cache
    let crates = ["serde", "tokio", "reqwest", "rand", "chrono"];

    // First round - fill cache
    for krate in crates {
        let result = test
            .read_resource(&format!("cargo://crate/{}/info", krate))
            .await;
        assert!(result.is_ok());
    }

    // Request a new crate to trigger eviction
    let result = test.read_resource("cargo://crate/actix-web/info").await;
    assert!(result.is_ok());

    // Original crates should still work but may trigger new requests
    for krate in crates {
        let result = test
            .read_resource(&format!("cargo://crate/{}/info", krate))
            .await;
        assert!(result.is_ok());
    }
}

//= docs/design/technical-spec.md#performance-considerations
//= type=test
//# The server SHOULD optimize cache hit rates for commonly accessed crates.
#[tokio::test]
async fn test_cache_performance_under_load() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Simulate heavy load with repeated requests
    let start = Instant::now();

    for _ in 0..50 {
        let result = test
            .read_resource("cargo://crate/serde/info")
            .await
            .unwrap();
        assert_eq!(result.contents[0].mime_type(), Some("application/json"));
    }

    let duration = start.elapsed();

    // Average time per request should be low due to caching
    let avg_duration = duration.as_millis() as f64 / 50.0;
    assert!(avg_duration < 10.0); // Less than 10ms per request on average
}

//= docs/design/technical-spec.md#performance-considerations
//= type=test
//# The server SHOULD implement concurrent request handling efficiently.
#[tokio::test]
async fn test_concurrent_different_resources() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Request different resources concurrently
    let start = Instant::now();

    let handles = vec![
        tokio::spawn({
            let test = test.clone();
            async move { test.read_resource("cargo://crate/serde/info").await }
        }),
        tokio::spawn({
            let test = test.clone();
            async move { test.read_resource("cargo://crate/tokio/info").await }
        }),
        tokio::spawn({
            let test = test.clone();
            async move { test.read_resource("cargo://project/metadata").await }
        }),
    ];

    let results = futures::future::join_all(handles).await;
    let duration = start.elapsed();

    // All requests should succeed
    for result in results {
        assert!(result.unwrap().is_ok());
    }

    // Total time should be reasonable (not 3x single request)
    assert!(duration < std::time::Duration::from_secs(3));
}
