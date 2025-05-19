use super::{ResourceContentsExt, Test, TestContext};
use std::sync::Arc;

//= docs/design/technical-spec.md#security-considerations
//# The server MUST validate all input parameters to prevent command injection.
#[tokio::test]
async fn test_command_injection_prevention() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Test various command injection attempts
    let injection_attempts = [
        "serde; rm -rf /",
        "serde && echo malicious",
        "serde || true",
        "serde | cat /etc/passwd",
        "serde` rm file `",
        r#"serde" && echo hack"#,
        "serde' && echo hack",
        "serde/**/../../etc",
    ];

    for attempt in injection_attempts {
        let result = test
            .read_resource(&format!("cargo://crate/{}/info", attempt))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("invalid") || err.to_string().contains("not found"),
            "Command injection attempt should fail: {}",
            attempt
        );
    }
}

//= docs/design/technical-spec.md#security-considerations
//# The server MUST handle file paths securely to prevent path traversal attacks.
#[tokio::test]
async fn test_path_traversal_prevention() {
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

    // Test path traversal attempts
    let traversal_attempts = [
        "../../../etc/passwd",
        "..\\..\\Windows\\System32",
        "%2e%2e%2f%2e%2e%2f", // URL encoded ../..
        "test/../../secret",
        "./././../other",
        "\\..\\..\\etc\\shadow",
    ];

    for attempt in traversal_attempts {
        let result = test
            .read_resource(&format!("cargo://crate/{}/info", attempt))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("invalid") || err.to_string().contains("not found"),
            "Path traversal attempt should fail: {}",
            attempt
        );
    }
}

//= docs/design/technical-spec.md#security-considerations
//# The server SHOULD implement rate limiting for resource-intensive operations.
#[tokio::test]
async fn test_resource_intensive_operations() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Test rapid documentation requests
    let mut success_count = 0;
    for _ in 0..100 {
        let result = test.read_resource("cargo://crate/serde/docs").await;

        if result.is_ok() {
            success_count += 1;
        }
    }

    // Some requests should be rate limited
    assert!(success_count < 100);
}

//= docs/design/technical-spec.md#security-considerations
//# The server SHOULD implement memory limits for cache storage.
#[tokio::test]
async fn test_memory_limits() {
    let ctx = Arc::new(TestContext::new().unwrap());

    // Create a test crate with large documentation
    ctx.file(
        "Cargo.toml",
        r#"
        [package]
        name = "large-crate"
        version = "0.1.0"
        edition = "2024"
        "#,
    )
    .unwrap();

    // Generate large documentation
    let mut large_doc = String::new();
    for i in 0..10000 {
        large_doc.push_str(&format!(
            "/// Doc comment {}\npub fn test_{}() {{}}\n",
            i, i
        ));
    }
    ctx.file("src/lib.rs", &large_doc).unwrap();

    let test = Test::start(ctx).await.unwrap();

    // Request should succeed but with limited response size
    let result = test
        .read_resource("cargo://crate/large-crate/docs")
        .await
        .unwrap();

    let doc = result.contents[0].as_text().unwrap();
    assert!(doc.len() < 10_000_000); // Response should be reasonably sized
}

//= docs/design/technical-spec.md#security-considerations
//# The server MUST validate all input parameters to prevent command injection.
#[tokio::test]
async fn test_version_parameter_validation() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Test malicious version parameters
    let bad_versions = [
        "1.0.0; rm -rf /",
        "1.0.0 && echo hack",
        "../../../1.0.0",
        "$(touch bad)",
        "`rm file`",
        "latest || true",
    ];

    for version in bad_versions {
        let result = test
            .read_resource(&format!("cargo://crate/serde/docs?version={}", version))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("invalid version"),
            "Invalid version should fail: {}",
            version
        );
    }
}

//= docs/design/technical-spec.md#security-considerations
//# The server MUST handle file paths securely to prevent path traversal attacks.
#[tokio::test]
async fn test_workspace_path_security() {
    let ctx = Arc::new(TestContext::new().unwrap());

    // Create a workspace with potential security issues
    ctx.file(
        "Cargo.toml",
        r#"
        [workspace]
        members = [
            "safe-crate",
            "../../../etc",  # Attempt path traversal
            "malicious;rm -rf /",  # Attempt command injection
            "\\..\\Windows",  # Windows path traversal
        ]
        "#,
    )
    .unwrap();

    ctx.file(
        "safe-crate/Cargo.toml",
        r#"
        [package]
        name = "safe-crate"
        version = "0.1.0"
        edition = "2024"
        "#,
    )
    .unwrap();

    ctx.file("safe-crate/src/lib.rs", "").unwrap();

    let test = Test::start(ctx).await.unwrap();

    // Should only process safe paths
    let result = test
        .read_resource("cargo://project/metadata")
        .await
        .unwrap();
    let metadata = result.contents[0].as_text().unwrap();

    // Only safe-crate should be included
    assert!(metadata.contains("safe-crate"));
    assert!(!metadata.contains("etc"));
    assert!(!metadata.contains("Windows"));
    assert!(!metadata.contains("malicious"));
}
