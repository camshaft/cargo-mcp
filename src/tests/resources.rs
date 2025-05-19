use super::{ResourceContentsExt, Test, TestContext};
use rmcp::model::ResourceContents;
use std::sync::Arc;

//= docs/design/technical-spec.md#crate-documentation-resource
//= type=test
//# The server MUST provide a resource at path `crate/{name}/docs`.

//= docs/design/technical-spec.md#crate-documentation-resource
//= type=test
//# The server MUST accept an optional version parameter in the query string.

//= docs/design/technical-spec.md#crate-documentation-resource
//= type=test
//# The server MUST generate rustdoc JSON using the nightly toolchain.

//= docs/design/technical-spec.md#crate-documentation-resource
//= type=test
//# The server MUST parse the rustdoc JSON using the public-api crate.

#[tokio::test]
async fn test_crate_docs_resource() {
    let ctx = Arc::new(TestContext::new().unwrap());

    // Create a simple test crate
    ctx.file(
        "Cargo.toml",
        r#"
        [package]
        name = "test-crate"
        version = "0.1.0"
        edition = "2024"

        [dependencies]
        "#,
    )
    .unwrap();

    ctx.file(
        "src/lib.rs",
        r#"
        //! Test crate documentation
        
        /// A test function
        /// 
        /// # Examples
        /// ```
        /// assert_eq!(test_crate::add(1, 2), 3);
        /// ```
        pub fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        /// A test struct
        #[derive(Debug)]
        pub struct Point {
            /// X coordinate
            pub x: f64,
            /// Y coordinate
            pub y: f64,
        }

        /// A test trait
        pub trait Shape {
            /// Calculate area
            fn area(&self) -> f64;
        }
        "#,
    )
    .unwrap();

    let test = Test::start(ctx).await.unwrap();

    // Test basic documentation request
    let result = test
        .read_resource("cargo://crate/test-crate/docs")
        .await
        .unwrap();
    let content: &ResourceContents = &result.contents[0];

    // Verify content type
    assert_eq!(content.mime_type(), Some("application/json"));

    // Parse response
    let doc: serde_json::Value = serde_json::from_str(content.as_text().unwrap()).unwrap();

    // Verify structure matches our type definitions
    let expected = serde_json::json!({
        "public_api": [
            {
                "name": "add",
                "kind": "function",
                "visibility": "public",
                "documentation": "A test function\n\n# Examples\n```\nassert_eq!(test_crate::add(1, 2), 3);\n```",
                "attributes": []
            },
            {
                "name": "Point",
                "kind": "type",
                "visibility": "public",
                "documentation": "A test struct",
                "attributes": [
                    {
                        "name": "derive",
                        "args": "Debug"
                    }
                ]
            },
            {
                "name": "Shape",
                "kind": "trait",
                "visibility": "public",
                "documentation": "A test trait",
                "attributes": []
            }
        ],
        "modules": [],
        "types": [
            {
                "name": "Point",
                "documentation": "A test struct",
                "kind": "struct",
                "attributes": [
                    {
                        "name": "derive",
                        "args": "Debug"
                    }
                ]
            }
        ],
        "traits": [
            {
                "name": "Shape",
                "documentation": "A test trait",
                "items": ["area"],
                "attributes": []
            }
        ]
    });

    assert_json_matches!(doc, expected);

    // Test version parameter
    let result = test
        .read_resource("cargo://crate/test-crate/docs?version=0.1.0")
        .await
        .unwrap();
    let content: &ResourceContents = &result.contents[0];
    let doc: serde_json::Value = serde_json::from_str(content.as_text().unwrap()).unwrap();
    assert_json_matches!(doc, expected);

    // Verify nightly toolchain usage
    let nightly_version = public_api::MINIMUM_NIGHTLY_RUST_VERSION;
    assert!(
        std::process::Command::new("rustup")
            .args(["toolchain", "list"])
            .output()
            .unwrap()
            .stdout
            .into_iter()
            .map(|b| b as char)
            .collect::<String>()
            .contains(nightly_version)
    );
}

//= docs/design/technical-spec.md#crate-information-resource
//= type=test
//# The server MUST provide a resource at path `crate/{name}/info`.

//= docs/design/technical-spec.md#crate-information-resource
//= type=test
//# The server MUST execute the `cargo info` command to retrieve crate information.

//= docs/design/technical-spec.md#crate-information-resource
//= type=test
//# The server MUST parse and return the latest version of the crate.

//= docs/design/technical-spec.md#crate-information-resource
//= type=test
//# The server MUST return all available versions of the crate.

//= docs/design/technical-spec.md#crate-information-resource
//= type=test
//# The server MUST return all available features and their descriptions.

//= docs/design/technical-spec.md#crate-information-resource
//= type=test
//# The server MUST return the crate's dependencies.

#[tokio::test]
async fn test_crate_info_resource() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Test with serde crate as it's well-known and stable
    let result = test
        .read_resource("cargo://crate/serde/info")
        .await
        .unwrap();
    let content: &ResourceContents = &result.contents[0];

    assert_eq!(content.mime_type(), Some("application/json"));

    let info: serde_json::Value = serde_json::from_str(content.as_text().unwrap()).unwrap();

    // Verify structure
    assert!(info.get("latest_version").is_some());
    assert!(info.get("all_versions").is_some());
    assert!(info.get("features").is_some());
    assert!(info.get("dependencies").is_some());
    assert!(info.get("description").is_some());
    assert!(info.get("repository").is_some());

    // Verify version format
    let version = info["latest_version"].as_str().unwrap();
    assert!(version.chars().all(|c| c.is_ascii_digit() || c == '.'));

    // Verify features
    let features = info["features"].as_object().unwrap();
    assert!(features.contains_key("derive"));
    assert!(features.contains_key("std"));
}

//= docs/design/technical-spec.md#project-metadata-resource
//= type=test
//# The server MUST provide a resource at path `project/metadata`.

//= docs/design/technical-spec.md#project-metadata-resource
//= type=test
//# The server MUST execute the `cargo metadata` command with format version 1.

//= docs/design/technical-spec.md#project-metadata-resource
//= type=test
//# The server MUST return workspace member information.

//= docs/design/technical-spec.md#project-metadata-resource
//= type=test
//# The server MUST return dependency information.

//= docs/design/technical-spec.md#project-metadata-resource
//= type=test
//# The server MUST return target information.

//= docs/design/technical-spec.md#project-metadata-resource
//= type=test
//# The server MUST return feature information.

#[tokio::test]
async fn test_project_metadata_resource() {
    let ctx = Arc::new(TestContext::new().unwrap());

    // Create a workspace with multiple crates
    ctx.file(
        "Cargo.toml",
        r#"
        [workspace]
        members = ["crate-a", "crate-b"]
        "#,
    )
    .unwrap();

    ctx.file(
        "crate-a/Cargo.toml",
        r#"
        [package]
        name = "crate-a"
        version = "0.1.0"
        edition = "2024"

        [dependencies]
        serde = { version = "1.0", features = ["derive"] }
        "#,
    )
    .unwrap();

    ctx.file(
        "crate-b/Cargo.toml",
        r#"
        [package]
        name = "crate-b"
        version = "0.1.0"
        edition = "2024"

        [dependencies]
        crate-a = { path = "../crate-a" }
        "#,
    )
    .unwrap();

    ctx.file("crate-a/src/lib.rs", "").unwrap();
    ctx.file("crate-b/src/lib.rs", "").unwrap();

    let test = Test::start(ctx).await.unwrap();

    let result = test
        .read_resource("cargo://project/metadata")
        .await
        .unwrap();
    let content = &result.contents[0];

    assert_eq!(content.mime_type(), Some("application/json"));

    let metadata: serde_json::Value = serde_json::from_str(content.as_text().unwrap()).unwrap();

    // Verify workspace members
    let members = metadata["workspace_members"].as_array().unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.iter().any(|m| m["name"] == "crate-a"));
    assert!(members.iter().any(|m| m["name"] == "crate-b"));

    // Verify dependencies
    let deps = metadata["dependencies"].as_array().unwrap();
    assert!(deps.iter().any(|d| d["name"] == "serde"));
    assert!(deps.iter().any(|d| d["name"] == "crate-a"));

    // Verify targets
    let targets = metadata["targets"].as_array().unwrap();
    assert!(!targets.is_empty());

    // Verify features
    let features = metadata["features"].as_object().unwrap();
    assert!(!features.is_empty());
}
