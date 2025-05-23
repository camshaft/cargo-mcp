// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::types::{ItemDoc, ItemKind};
use rustdoc_types::Crate;
use std::{collections::HashMap, path::Path, sync::Arc};
use thiserror::Error;
use tokio::sync::Mutex;

pub const NIGHTLY_VERSION: &str = "nightly-2025-05-09";

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to generate rustdoc JSON: {0}")]
    DocGenFailed(String),

    #[error("Failed to parse rustdoc JSON: {0}")]
    ParseFailed(String),

    #[error("Item not found: {0}")]
    ItemNotFound(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

#[derive(Default)]
pub struct RustdocProvider {
    cache: Arc<Mutex<HashMap<String, Arc<Crate>>>>,
}

impl RustdocProvider {
    pub fn new() -> Result<Self> {
        // Install nightly toolchain at initialization
        rustup_toolchain::install(NIGHTLY_VERSION).map_err(|e| {
            Error::DocGenFailed(format!("Failed to install nightly toolchain: {}", e))
        })?;

        Ok(Self::default())
    }

    pub async fn get_crate_docs(&self, _name: &str, _version: Option<&str>) -> Result<Arc<Crate>> {
        // TODO: Implement crate docs generation for crates.io crates
        todo!()
    }

    pub async fn get_workspace_docs(&self, path: &Path) -> Result<Arc<Crate>> {
        // For workspace docs, always regenerate since they can change frequently
        let krate = self.generate_rustdoc_json(path, true)?;
        Ok(Arc::new(krate))
    }

    pub fn resolve_item(&self, krate: &Crate, _path: &str) -> Result<ItemDoc> {
        // Find the root module
        let root_id = krate.root;
        let root = &krate.index[&root_id];

        // For now, just return basic info about the root module
        Ok(ItemDoc {
            name: root.name.clone().unwrap_or_default(),
            kind: ItemKind::Module,
            docs: root.docs.clone(),
            implemented_traits: vec![],
            methods: vec![],
            fields: vec![],
            variants: vec![],
        })
    }

    fn generate_rustdoc_json(&self, path: &Path, is_workspace: bool) -> Result<Crate> {
        // If path is a directory, append Cargo.toml
        let manifest_path = if path.ends_with("Cargo.toml") {
            path.to_path_buf()
        } else {
            path.join("Cargo.toml")
        };

        // Generate rustdoc JSON
        let json_path = rustdoc_json::Builder::default()
            .toolchain(NIGHTLY_VERSION)
            .manifest_path(&manifest_path)
            .document_private_items(is_workspace) // Include private items
            .build()
            .map_err(|e| Error::DocGenFailed(format!("Failed to generate rustdoc JSON: {}", e)))?;

        // Parse the JSON
        let json_str = std::fs::read_to_string(&json_path)
            .map_err(|e| Error::DocGenFailed(format!("Failed to read JSON file: {}", e)))?;

        eprintln!("{}", &json_str[..20.min(json_str.len())]);

        let mut deserializer = serde_json::Deserializer::from_str(&json_str);
        deserializer.disable_recursion_limit();
        let krate = serde::de::Deserialize::deserialize(&mut deserializer)
            .map_err(|e| Error::ParseFailed(format!("Failed to parse rustdoc JSON: {}", e)))?;

        Ok(krate)
    }
}
