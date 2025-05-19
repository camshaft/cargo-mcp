use super::*;
use std::sync::Arc;
use tokio::test;

//= docs/design/technical-spec.md#general
//= type=test
//# The server MUST implement the Model Context Protocol (MCP) specification.
#[test]
async fn test_mcp_protocol_implementation() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Just verify we can connect and disconnect cleanly
    test.cancel().await.unwrap();
}

//= docs/design/technical-spec.md#general
//= type=test
//# The server MUST provide access to documentation through the public-api crate's analysis of rustdoc JSON output.
#[test]
async fn test_complete_documentation_workflow() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx.clone()).await.unwrap();

    // Create a crate with various Rust features
    ctx.file(
        "Cargo.toml",
        r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2024"

[features]
default = ["feature1"]
feature1 = []
feature2 = ["feature1"]
        "#,
    )
    .unwrap();

    ctx.file(
        "src/lib.rs",
        r#"
//! Test crate documentation
//! 
//! This crate demonstrates various Rust features.

/// A generic data structure
#[derive(Debug)]
pub struct Container<T> {
    /// The contained value
    value: T,
}

impl<T> Container<T> {
    /// Creates a new container
    pub fn new(value: T) -> Self {
        Self { value }
    }

    /// Gets a reference to the contained value
    pub fn get(&self) -> &T {
        &self.value
    }
}

/// A trait for types that can be displayed
pub trait Displayable {
    /// Converts the value to a string
    fn to_string(&self) -> String;
}

#[cfg(feature = "feature1")]
pub mod feature1 {
    //! Feature 1 implementation

    /// A function enabled by feature1
    pub fn feature1_function() -> &'static str {
        "feature1"
    }
}

#[cfg(feature = "feature2")]
pub mod feature2 {
    //! Feature 2 implementation

    /// A function enabled by feature2
    pub fn feature2_function() -> &'static str {
        "feature2"
    }
}
        "#,
    )
    .unwrap();

    // Test documentation resource
    let docs = test
        .read_resource("cargo://crate/test-crate/docs")
        .await
        .unwrap();
    let doc_content = docs.contents[0].as_text().unwrap();

    // Verify documentation content
    assert!(doc_content.contains("Container"));
    assert!(doc_content.contains("Displayable"));
    assert!(doc_content.contains("feature1_function"));
    assert!(!doc_content.contains("feature2_function")); // Not in default features

    // Test crate info resource
    let info = test
        .read_resource("cargo://crate/test-crate/info")
        .await
        .unwrap();
    let info_content = info.contents[0].as_text().unwrap();

    // Verify crate information
    assert!(info_content.contains("\"latest_version\":\"0.1.0\""));
    assert!(info_content.contains("feature1"));
    assert!(info_content.contains("feature2"));

    // Test project metadata resource
    let metadata = test
        .read_resource("cargo://project/metadata")
        .await
        .unwrap();
    let metadata_content = metadata.contents[0].as_text().unwrap();

    // Verify project metadata
    assert!(metadata_content.contains("test-crate"));
    assert!(metadata_content.contains("0.1.0"));
    assert!(metadata_content.contains("feature1"));
    assert!(metadata_content.contains("feature2"));

    // Test version-specific documentation
    let versioned_docs = test
        .read_resource("cargo://crate/test-crate/docs?version=0.1.0")
        .await
        .unwrap();

    // Should match non-versioned docs since it's the only version
    assert_eq!(versioned_docs.contents[0].as_text().unwrap(), doc_content);

    test.cancel().await.unwrap();
}

//= docs/design/technical-spec.md#general
//= type=test
//# The server MUST handle concurrent requests safely.

//= docs/design/technical-spec.md#general
//= type=test
//# The server MUST support caching of documentation and crate information.

#[test]
async fn test_concurrent_mixed_requests() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx.clone()).await.unwrap();

    // Create test crate
    ctx.file(
        "Cargo.toml",
        r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = "1.0"
        "#,
    )
    .unwrap();

    ctx.file("src/lib.rs", "").unwrap();

    // Make concurrent requests for different resources
    let (docs, info, metadata, serde_info) = tokio::join!(
        test.read_resource("cargo://crate/test-crate/docs"),
        test.read_resource("cargo://crate/test-crate/info"),
        test.read_resource("cargo://project/metadata"),
        test.read_resource("cargo://crate/serde/info")
    );

    // All requests should succeed
    assert!(docs.is_ok());
    assert!(info.is_ok());
    assert!(metadata.is_ok());
    assert!(serde_info.is_ok());

    // Verify responses are properly structured
    let docs = docs.unwrap();
    let info = info.unwrap();
    let metadata = metadata.unwrap();
    let serde_info = serde_info.unwrap();

    assert_eq!(docs.contents[0].mime_type(), Some("application/json"));
    assert_eq!(info.contents[0].mime_type(), Some("application/json"));
    assert_eq!(metadata.contents[0].mime_type(), Some("application/json"));
    assert_eq!(serde_info.contents[0].mime_type(), Some("application/json"));

    // Make the same requests again to test caching
    let (docs2, info2, metadata2, serde_info2) = tokio::join!(
        test.read_resource("cargo://crate/test-crate/docs"),
        test.read_resource("cargo://crate/test-crate/info"),
        test.read_resource("cargo://project/metadata"),
        test.read_resource("cargo://crate/serde/info")
    );

    // Cached responses should match originals
    assert_eq!(
        docs.contents[0].as_text().unwrap(),
        docs2.unwrap().contents[0].as_text().unwrap()
    );
    assert_eq!(
        info.contents[0].as_text().unwrap(),
        info2.unwrap().contents[0].as_text().unwrap()
    );
    assert_eq!(
        metadata.contents[0].as_text().unwrap(),
        metadata2.unwrap().contents[0].as_text().unwrap()
    );
    assert_eq!(
        serde_info.contents[0].as_text().unwrap(),
        serde_info2.unwrap().contents[0].as_text().unwrap()
    );

    test.cancel().await.unwrap();
}

//= docs/design/technical-spec.md#general
//= type=test
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
//= type=test
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
