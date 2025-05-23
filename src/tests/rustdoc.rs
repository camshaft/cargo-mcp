// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use super::TestContext;
use crate::{providers::rustdoc::RustdocProvider, types::ItemKind};
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

    let provider = RustdocProvider::new().unwrap();

    // Test rustdoc generation
    let krate = provider.get_workspace_docs(root).await.unwrap();

    // Test root module resolution
    let item = provider.resolve_item(&krate, "test_crate").unwrap();
    assert_eq!(item.name, "test_crate");
    assert!(matches!(item.kind, ItemKind::Module));

    // Print the index to help with development
    eprintln!("Index items:");
    for (id, item) in krate.index.iter() {
        eprintln!("  {:?}: {:?}", id, item);
    }
}
