// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use super::{Test, TestContext};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_workspace_single_crate() {
    let ctx = Arc::new(TestContext::new().unwrap());

    // Create a crate with various Rust features
    ctx.file(
        "Cargo.toml",
        r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2024"
        "#,
    );

    ctx.file("src/lib.rs", "");

    let root = ctx.root().to_string_lossy().to_string();
    eprintln!("Single crate test root path: {}", root);

    let test = Test::start(ctx).await.unwrap();

    let result = test
        .call_tool("workspace_crates", vec![("directory", json!(root))])
        .await
        .unwrap();

    let content = &result.content[0];
    let text = &content.as_text().unwrap().text;
    let value: serde_json::Value = serde_json::from_str(text).unwrap();
    let crates = value.as_array().unwrap();

    assert_eq!(crates.len(), 1);
    assert_eq!(crates[0]["name"], "test-crate");
}

#[tokio::test]
async fn test_workspace_multiple_crates() {
    let ctx = Arc::new(TestContext::new().unwrap());

    // Create a workspace with multiple crates
    ctx.file(
        "Cargo.toml",
        r#"
[workspace]
members = ["test-crate-1", "test-crate-2"]
        "#,
    );

    ctx.file(
        "test-crate-1/Cargo.toml",
        r#"
[package]
name = "test-crate-1"
version = "0.1.0"
edition = "2024"
        "#,
    );
    ctx.file("test-crate-1/src/lib.rs", "");

    ctx.file(
        "test-crate-2/Cargo.toml",
        r#"
[package]
name = "test-crate-2"
version = "0.1.0"
edition = "2024"
        "#,
    );
    ctx.file("test-crate-2/src/lib.rs", "");

    let root = ctx.root().to_string_lossy().to_string();
    eprintln!("Multiple crates test root path: {}", root);
    let test = Test::start(ctx).await.unwrap();

    let result = test
        .call_tool("workspace_crates", vec![("directory", json!(root))])
        .await
        .unwrap();

    let content = &result.content[0];
    let text = &content.as_text().unwrap().text;
    let value: serde_json::Value = serde_json::from_str(text).unwrap();
    let crates = value.as_array().unwrap();

    assert_eq!(crates.len(), 2);
    assert!(crates.iter().any(|c| c["name"] == "test-crate-1"));
    assert!(crates.iter().any(|c| c["name"] == "test-crate-2"));
}

#[tokio::test]
async fn test_crates_io_latest_version() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Test with serde crate as it's well-known and stable
    let result = test
        .call_tool(
            "crates_io_latest_version",
            vec![("crate_name", json!("serde"))],
        )
        .await
        .unwrap();

    let content = &result.content[0];
    let text = &content.as_text().unwrap().text;
    let value: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(value["name"], "serde");
    assert!(
        value["version"]
            .as_str()
            .unwrap()
            .chars()
            .next()
            .unwrap()
            .is_numeric()
    );
}

#[tokio::test]
async fn test_crates_io_versions() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Test with serde crate as it's well-known and stable
    let result = test
        .call_tool("crates_io_versions", vec![("crate_name", json!("serde"))])
        .await
        .unwrap();

    let content = &result.content[0];
    let text = &content.as_text().unwrap().text;
    let value: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(value["name"], "serde");
    assert!(value["versions"].is_array());
    let versions = value["versions"].as_array().unwrap();
    assert!(!versions.is_empty());

    // Check version object structure
    let version = &versions[0];
    assert!(version["version"].is_string());
    assert!(version["yanked"].is_boolean());
    assert!(version["msrv"].is_null() || version["msrv"].is_string());
}

#[tokio::test]
async fn test_crates_io_features() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Test with serde crate as it's well-known and stable
    let result = test
        .call_tool(
            "crates_io_features",
            vec![("crate_name", json!("serde")), ("version", json!(null))],
        )
        .await
        .unwrap();

    let content = &result.content[0];
    let text = &content.as_text().unwrap().text;
    let value: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(value["name"], "serde");
    assert_eq!(value["version"], "latest");
    assert!(value["features"].is_array());
}

#[tokio::test]
async fn test_crates_io_features_with_version() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // First get the versions
    let versions = test
        .call_tool("crates_io_versions", vec![("crate_name", json!("serde"))])
        .await
        .unwrap();

    let content = &versions.content[0];
    let text = &content.as_text().unwrap().text;
    let value: serde_json::Value = serde_json::from_str(text).unwrap();
    let version = value["versions"].as_array().unwrap()[0]["version"]
        .as_str()
        .unwrap();

    // Now test features for a specific version
    let result = test
        .call_tool(
            "crates_io_features",
            vec![("crate_name", json!("serde")), ("version", json!(version))],
        )
        .await
        .unwrap();

    let content = &result.content[0];
    let text = &content.as_text().unwrap().text;
    let value: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(value["name"], "serde");
    assert_eq!(value["version"], version);
    assert!(value["features"].is_array());
}

#[tokio::test]
async fn test_crates_io_latest_version_invalid() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Test with an invalid crate name
    let result = test
        .call_tool(
            "crates_io_latest_version",
            vec![(
                "crate_name",
                json!("this-crate-definitely-does-not-exist-12345"),
            )],
        )
        .await
        .unwrap();

    assert!(result.is_error.unwrap());
}

#[tokio::test]
async fn test_crates_io_features_invalid() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Test with an invalid crate name
    let result = test
        .call_tool(
            "crates_io_features",
            vec![
                (
                    "crate_name",
                    json!("this-crate-definitely-does-not-exist-12345"),
                ),
                ("version", json!(null)),
            ],
        )
        .await
        .unwrap();

    assert!(result.is_error.unwrap());
}
