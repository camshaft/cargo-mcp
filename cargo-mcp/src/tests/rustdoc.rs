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
    let items = krate.search("test_crate", None);
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(&*item.path, "test_crate");
    assert!(matches!(item.kind, ItemKind::Module));
    assert_eq!(item.docs.as_deref(), Some("Test crate documentation."));

    // Test struct resolution
    let items = krate.search("TestStruct", None);
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(&*item.path, "test_crate::TestStruct");
    assert!(matches!(item.kind, ItemKind::Struct));
    assert_eq!(item.docs.as_deref(), Some("A test struct."));

    // Test function resolution
    let items = krate.search("new", None);
    assert!(!items.is_empty());
    let new_fn = items
        .iter()
        .find(|i| &*i.path == "test_crate::TestStruct::new")
        .unwrap();
    assert!(matches!(new_fn.kind, ItemKind::Function));
    assert_eq!(new_fn.docs.as_deref(), Some("Creates a new instance."));
}

#[tokio::test]
async fn test_rustdoc_crates_io() {
    let provider = RustdocProvider::new().unwrap();

    // Test crates.io docs
    let krate = provider.get_crate_docs("serde", Some("1.0")).await.unwrap();

    // Test root module
    let items = krate.search("serde", None);
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(&*item.path, "serde");
    assert!(matches!(item.kind, ItemKind::Module));

    // Test well-known items
    let items = krate.search("Serialize", None);
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(&*item.path, "serde::ser::Serialize");
    assert!(matches!(item.kind, ItemKind::Trait));

    let items = krate.search("Deserialize", None);
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(&*item.path, "serde::de::Deserialize");
    assert!(matches!(item.kind, ItemKind::Trait));

    // Test fuzzy matching
    let items = krate.search("serial", None);
    eprintln!("Fuzzy search 'serial' found {} items:", items.len());
    assert!(!items.is_empty());
    assert!(items.iter().any(|i| &*i.path == "serde::ser::Serialize"));

    // Test qualified paths
    let items = krate.search("ser::Serializer", None);
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(&*item.path, "serde::ser::Serializer");
    assert!(matches!(item.kind, ItemKind::Trait));

    let items = krate.search("de::Deserializer", None);
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(&*item.path, "serde::de::Deserializer");
    assert!(matches!(item.kind, ItemKind::Trait));
}
