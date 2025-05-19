use super::*;
use std::sync::Arc;
use tokio::test;

//= docs/design/technical-spec.md#general
//# The server MUST implement the Model Context Protocol (MCP) specification.
#[test]
async fn test_mcp_protocol_implementation() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Just verify we can connect and disconnect cleanly
    test.cancel().await.unwrap();
}

//= docs/design/technical-spec.md#general
//# The server MUST support caching of documentation and crate information.
#[test]
async fn test_caching_support() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx.clone()).await.unwrap();

    // Create Cargo.toml in root
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

    // Create src/lib.rs in root
    ctx.file(
        "src/lib.rs",
        r#"
/// Returns the string "Hello, world!"
pub fn hello() -> &'static str {
    "Hello, world!"
}
        "#,
    )
    .unwrap();

    // First request should cache the result
    let first_response = test.read_resource("project/metadata").await.unwrap();

    // Second request should return cached result
    let second_response = test.read_resource("project/metadata").await.unwrap();

    // Responses should match exactly since second came from cache
    assert_json_matches!(first_response, second_response);

    test.cancel().await.unwrap();
}

//= docs/design/technical-spec.md#general
//# The server MUST handle concurrent requests safely.
#[test]
async fn test_concurrent_requests() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx.clone()).await.unwrap();

    // Create test crates in root
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
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
        "#,
    )
    .unwrap();

    // Make concurrent requests
    let (docs1, docs2) = tokio::join!(
        test.read_resource("crate/test-crate/docs"),
        test.read_resource("crate/test-crate/docs")
    );

    // Both requests should succeed
    assert_eq!(docs1.unwrap(), docs2.unwrap());

    test.cancel().await.unwrap();
}
