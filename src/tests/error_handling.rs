use super::{ResourceContentsExt, Test, TestContext};
use std::sync::Arc;

//= docs/design/technical-spec.md#error-handling
//= type=test
//# The server MUST return appropriate error responses for all failure cases.

//= docs/design/technical-spec.md#error-handling
//= type=test
//# The server MUST return a "not found" error when a requested crate does not exist.

#[tokio::test]
async fn test_crate_not_found() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    let result = test
        .read_resource("cargo://crate/nonexistent-crate/info")
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

//= docs/design/technical-spec.md#error-handling
//= type=test
//# The server MUST return an "invalid parameters" error when an invalid version is specified.

#[tokio::test]
async fn test_invalid_version() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    let result = test
        .read_resource("cargo://crate/serde/docs?version=invalid-version")
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("invalid version"));
}

//= docs/design/technical-spec.md#security-considerations
//= type=test
//# The server MUST validate all input parameters to prevent command injection.
#[tokio::test]
async fn test_crate_name_validation() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Test with potentially dangerous crate names
    let dangerous_names = [
        "../../etc/passwd",
        "malicious;rm -rf /",
        "test$(touch bad)",
        "test`rm file`",
        r#"test"&&echo bad"#,
    ];

    for name in dangerous_names {
        let result = test
            .read_resource(&format!("cargo://crate/{}/info", name))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("invalid") || err.to_string().contains("not found"),
            "Expected validation error for name: {}",
            name
        );
    }
}

//= docs/design/technical-spec.md#error-handling
//= type=test
//# The server MUST return an "internal error" for command execution failures.

#[tokio::test]
async fn test_command_execution_failure() {
    let ctx = Arc::new(TestContext::new().unwrap());

    // Create an invalid manifest to trigger cargo command failure
    ctx.file(
        "Cargo.toml",
        r#"
        [package]
        name = "invalid-manifest"
        version = "not-a-version"
        "#,
    )
    .unwrap();

    let test = Test::start(ctx).await.unwrap();

    let result = test.read_resource("cargo://project/metadata").await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("internal error"));
}

//= docs/design/technical-spec.md#error-handling
//= type=test
//# The server MUST return an "internal error" for parsing failures.

#[tokio::test]
async fn test_parsing_failure() {
    let ctx = Arc::new(TestContext::new().unwrap());

    // Create a crate with invalid Rust code to trigger parsing failure
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
        // Invalid Rust code
        pub fn broken( -> {
        "#,
    )
    .unwrap();

    let test = Test::start(ctx).await.unwrap();

    let result = test.read_resource("cargo://crate/test-crate/docs").await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("internal error"));
}

//= docs/design/technical-spec.md#error-handling
//= type=test
//# The server MUST provide error messages that are helpful for debugging.

#[tokio::test]
async fn test_error_messages() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Test various error cases and verify messages are helpful
    let test_cases = [
        (
            "cargo://crate/nonexistent/info",
            "crate 'nonexistent' not found",
        ),
        (
            "cargo://crate/serde/docs?version=99.99.99",
            "version '99.99.99' not found",
        ),
        ("cargo://invalid/path", "invalid resource path"),
    ];

    for (resource, expected_msg) in test_cases {
        let result = test.read_resource(resource).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains(expected_msg),
            "Error '{}' should contain '{}'",
            err,
            expected_msg
        );
    }
}

//= docs/design/technical-spec.md#caching-requirements
//= type=test
//# The server MUST validate cache entries before returning them.
#[tokio::test]
async fn test_invalid_cache_entries() {
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

    // First request should succeed
    let result = test
        .read_resource("cargo://crate/test-crate/info")
        .await
        .unwrap();
    assert_eq!(result.contents[0].mime_type(), Some("application/json"));

    // Wait for cache to expire (configured in test context)
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Second request should still work (cache miss, fresh data)
    let result = test
        .read_resource("cargo://crate/test-crate/info")
        .await
        .unwrap();
    assert_eq!(result.contents[0].mime_type(), Some("application/json"));
}
