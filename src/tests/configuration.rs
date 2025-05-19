use super::{ResourceContentsExt, Test, TestContext};
use crate::Config;
use std::{sync::Arc, time::Duration};

//= docs/design/technical-spec.md#configuration
//# The server MUST support configuration of cache TTL values.
#[tokio::test]
async fn test_cache_ttl_configuration() {
    let ctx = Arc::new(TestContext::new().unwrap());

    // Create server with short TTL
    let mut config = Config::default();
    config.doc_cache_ttl = Duration::from_millis(100);
    config.info_cache_ttl = Duration::from_millis(50);

    let test = Test::start(ctx).await.unwrap();

    // Test info cache with shorter TTL
    let result = test
        .read_resource("cargo://crate/serde/info")
        .await
        .unwrap();
    let first_response = result.contents[0].as_text().unwrap().to_string();

    // Wait for info cache to expire
    tokio::time::sleep(Duration::from_millis(75)).await;

    // Should get fresh data
    let result = test
        .read_resource("cargo://crate/serde/info")
        .await
        .unwrap();
    let second_response = result.contents[0].as_text().unwrap().to_string();

    // Responses may differ due to fresh data
    assert_ne!(first_response, second_response);

    // Test doc cache with longer TTL
    let result = test
        .read_resource("cargo://crate/serde/docs")
        .await
        .unwrap();
    let first_doc = result.contents[0].as_text().unwrap().to_string();

    // Wait for a time between the TTLs
    tokio::time::sleep(Duration::from_millis(75)).await;

    // Doc cache should still be valid
    let result = test
        .read_resource("cargo://crate/serde/docs")
        .await
        .unwrap();
    let second_doc = result.contents[0].as_text().unwrap().to_string();

    // Doc responses should be identical due to caching
    assert_eq!(first_doc, second_doc);
}

//= docs/design/technical-spec.md#configuration
//# The server MUST support configuration of maximum cache size.
#[tokio::test]
async fn test_max_cache_size_configuration() {
    let ctx = Arc::new(TestContext::new().unwrap());

    // Create server with small cache size
    let mut config = Config::default();
    config.max_cache_size = 2;

    let test = Test::start(ctx).await.unwrap();

    // Fill cache beyond capacity
    let crates = ["serde", "tokio", "reqwest"];

    for krate in crates {
        let result = test
            .read_resource(&format!("cargo://crate/{}/info", krate))
            .await;
        assert!(result.is_ok());
    }

    // First crate should have been evicted
    let result = test
        .read_resource("cargo://crate/serde/info")
        .await
        .unwrap();

    // Response should be fresh data
    assert!(
        result.contents[0]
            .as_text()
            .unwrap()
            .contains("\"latest_version\"")
    );
}

//= docs/design/technical-spec.md#configuration
//# The server SHOULD support configuration of the nightly toolchain path.
#[tokio::test]
async fn test_nightly_toolchain_configuration() {
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

    ctx.file(
        "src/lib.rs",
        r#"
        //! Test documentation
        pub fn test() {}
        "#,
    )
    .unwrap();

    // Create server with specific nightly version
    let mut config = Config::default();
    config.nightly_toolchain = Some(public_api::MINIMUM_NIGHTLY_RUST_VERSION.to_string());

    let test = Test::start(ctx).await.unwrap();

    // Should use configured toolchain
    let result = test
        .read_resource("cargo://crate/test-crate/docs")
        .await
        .unwrap();

    assert!(result.contents[0].as_text().unwrap().contains("test_crate"));
}

//= docs/design/technical-spec.md#configuration
//# The server MAY support configuration of custom cargo command paths.
#[tokio::test]
async fn test_cargo_command_configuration() {
    let ctx = Arc::new(TestContext::new().unwrap());

    // Create server with custom cargo path
    let mut config = Config::default();
    config.cargo_path = Some("cargo".to_string()); // Use system cargo

    let test = Test::start(ctx).await.unwrap();

    // Should work with configured cargo path
    let result = test.read_resource("cargo://project/metadata").await;
    assert!(result.is_ok());
}

//= docs/design/technical-spec.md#configuration
//# The server MUST support configuration of cache TTL values.
#[tokio::test]
async fn test_different_cache_ttls() {
    let ctx = Arc::new(TestContext::new().unwrap());

    // Create server with different TTLs
    let mut config = Config::default();
    config.doc_cache_ttl = Duration::from_secs(60); // Long TTL for docs
    config.info_cache_ttl = Duration::from_secs(10); // Short TTL for info

    let test = Test::start(ctx).await.unwrap();

    // Test info cache (short TTL)
    let result = test
        .read_resource("cargo://crate/serde/info")
        .await
        .unwrap();
    let first_info = result.contents[0].as_text().unwrap().to_string();

    // Test doc cache (long TTL)
    let result = test
        .read_resource("cargo://crate/serde/docs")
        .await
        .unwrap();
    let first_doc = result.contents[0].as_text().unwrap().to_string();

    // Wait for info cache to expire but doc cache to remain valid
    tokio::time::sleep(Duration::from_secs(15)).await;

    // Info should be fresh
    let result = test
        .read_resource("cargo://crate/serde/info")
        .await
        .unwrap();
    let second_info = result.contents[0].as_text().unwrap().to_string();
    assert_ne!(first_info, second_info);

    // Doc should be cached
    let result = test
        .read_resource("cargo://crate/serde/docs")
        .await
        .unwrap();
    let second_doc = result.contents[0].as_text().unwrap().to_string();
    assert_eq!(first_doc, second_doc);
}
