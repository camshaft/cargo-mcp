// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use super::TestContext;
use crate::{
    providers::{crates_io, rustdoc::RustdocProvider},
    types::ItemKind,
};
use std::sync::Arc;

#[tokio::test]
async fn test_rustdoc_workspace() {
    let ctx = Arc::new(TestContext::new().unwrap());

    // Create a simple crate
    ctx.file(
        "Cargo.toml",
        r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2024"

[lib]
path = "src/lib.rs"
        "#,
    );

    ctx.file(
        "src/lib.rs",
        r#"
//! Test crate documentation.

/// A test struct.
pub struct TestStruct {
    /// A test field.
    pub field: String,
}

impl TestStruct {
    /// Creates a new instance.
    pub fn new(field: String) -> Self {
        Self { field }
    }
}
        "#,
    );

    let root = ctx.root();
    eprintln!("Test root path: {}", root.display());

    let crates_io = crates_io::CratesIoProvider::new().unwrap();
    let provider = RustdocProvider::new(crates_io).unwrap();

    // Test rustdoc generation
    let krate = provider.get_workspace_docs(root).await.unwrap();

    // Test root module resolution
    let item = provider.resolve_item(&krate, "test_crate").unwrap();
    assert_eq!(item.name, "test_crate");
    assert!(matches!(item.kind, ItemKind::Module));
    assert_eq!(item.docs.as_deref(), Some("Test crate documentation."));

    // Test struct resolution
    let item = provider.resolve_item(&krate, "TestStruct").unwrap();
    assert_eq!(item.name, "TestStruct");
    assert!(matches!(item.kind, ItemKind::Struct));
    assert_eq!(item.docs.as_deref(), Some("A test struct."));

    // Test function resolution
    let item = provider.resolve_item(&krate, "new").unwrap();
    assert_eq!(item.name, "new");
    assert!(matches!(item.kind, ItemKind::Function));
    assert_eq!(item.docs.as_deref(), Some("Creates a new instance."));

    // Test crates.io docs
    let krate = provider.get_crate_docs("serde", Some("1.0")).await.unwrap();

    // Test root module
    let item = provider.resolve_item(&krate, "serde").unwrap();
    assert_eq!(item.name, "serde");
    assert!(matches!(item.kind, ItemKind::Module));

    // Test well-known items
    let item = provider.resolve_item(&krate, "Serialize").unwrap();
    assert_eq!(item.name, "Serialize");
    assert!(matches!(item.kind, ItemKind::Trait));

    let item = provider.resolve_item(&krate, "Deserialize").unwrap();
    assert_eq!(item.name, "Deserialize");
    assert!(matches!(item.kind, ItemKind::Trait));

    // Print the index to help with development
    // eprintln!("Index items:");
    // for (id, item) in krate.index.iter() {
    //     eprintln!("  {:?}: {:?}", id, item);
    // }
}
