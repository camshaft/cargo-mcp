// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    providers::crates_io::CratesIoProvider,
    types::{ItemDoc, ItemKind},
};
use rustdoc_types::{Crate, ItemEnum};
use semver::{Version, VersionReq};
use std::{collections::HashMap, path::Path, str::FromStr, sync::Arc};
use tokio::sync::Mutex;

pub const NIGHTLY_VERSION: &str = "nightly-2025-05-09";

type Error = Box<dyn std::error::Error + Send + Sync>;
type Result<T = (), E = Error> = std::result::Result<T, E>;

pub struct RustdocProvider {
    cache: Arc<Mutex<HashMap<String, Arc<Crate>>>>,
    crates_io: CratesIoProvider,
}

impl RustdocProvider {
    pub fn new(crates_io: CratesIoProvider) -> Result<Self> {
        // Install nightly toolchain at initialization
        rustup_toolchain::install(NIGHTLY_VERSION)
            .map_err(|e| format!("Failed to install nightly toolchain: {}", e))?;

        Ok(Self {
            cache: Default::default(),
            crates_io,
        })
    }

    pub async fn get_crate_docs(&self, name: &str, version: Option<&str>) -> Result<Arc<Crate>> {
        let cache_key = format!("{}:{}", name, version.unwrap_or("*"));

        // Check cache first
        let cache = self.cache.lock().await;
        if let Some(krate) = cache.get(&cache_key) {
            return Ok(krate.clone());
        }
        drop(cache); // Release lock before generating docs

        // Create temporary directory for the crate
        let temp_dir =
            tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;

        let version = if let Some(version_req) = version {
            // Get crate info and resolve version
            let krate = self
                .crates_io
                .fetch(name)
                .await
                .map_err(|e| format!("Failed to fetch crate info: {}", e))?;

            let req = VersionReq::from_str(version_req)
                .map_err(|e| format!("Invalid version requirement: {}", e))?;

            let version = krate
                .versions()
                .iter()
                .filter(|v| !v.is_yanked())
                .filter_map(|v| Version::from_str(v.version()).ok().map(|ver| (v, ver)))
                .filter(|(_, ver)| req.matches(ver))
                .max_by(|(_, a), (_, b)| a.cmp(b))
                .map(|(v, _)| v)
                .ok_or_else(|| {
                    format!("No matching version found for requirement {}", version_req)
                })?;

            Some(version.version().to_string())
        } else {
            None
        };

        // Download and extract crate
        let url = self
            .crates_io
            .get_download_url(name, version.as_deref())
            .await
            .map_err(|e| format!("Failed to get download URL: {}", e))?;

        let response = reqwest::get(&url)
            .await
            .map_err(|e| format!("Failed to download crate: {}", e))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let tar = flate2::read::GzDecoder::new(&bytes[..]);
        let mut archive = tar::Archive::new(tar);
        archive
            .unpack(temp_dir.path())
            .map_err(|e| format!("Failed to extract crate: {}", e))?;

        // Find the package directory (it's usually inside a folder named {name}-{version})
        let entries = std::fs::read_dir(temp_dir.path())
            .map_err(|e| format!("Failed to read temp dir: {}", e))?;

        let pkg_dir = entries
            .filter_map(|e| e.ok())
            .find(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .ok_or_else(|| "No package directory found".to_string())?
            .path();

        // Generate rustdoc JSON
        let krate = Arc::new(self.generate_rustdoc_json(&pkg_dir, false)?);

        // Cache the result
        let mut cache = self.cache.lock().await;
        cache.insert(cache_key, krate.clone());

        Ok(krate)
    }

    pub async fn get_workspace_docs(&self, path: &Path) -> Result<Arc<Crate>> {
        // For workspace docs, always regenerate since they can change frequently
        let krate = self.generate_rustdoc_json(path, true)?;
        Ok(Arc::new(krate))
    }

    pub fn resolve_item(&self, krate: &Crate, path: &str) -> Result<ItemDoc> {
        // Find the item in the index
        let item = krate
            .index
            .values()
            .find(|item| item.name.as_deref() == Some(path))
            .ok_or_else(|| format!("Item {path} not found"))?;

        // Convert to our ItemDoc format
        let kind = match &item.inner {
            ItemEnum::Module(_) => ItemKind::Module,
            ItemEnum::Struct(_) => ItemKind::Struct,
            ItemEnum::Enum(_) => ItemKind::Enum,
            ItemEnum::Function(_) => ItemKind::Function,
            ItemEnum::Trait(_) => ItemKind::Trait,
            ItemEnum::TypeAlias(_) => ItemKind::Type,
            ItemEnum::Constant { .. } => ItemKind::Constant,
            _ => {
                return Err(format!("Unsupported item kind: {:?}", item.inner).into());
            }
        };

        Ok(ItemDoc {
            name: item.name.clone().unwrap_or_default(),
            kind,
            docs: item.docs.clone(),
            implemented_traits: vec![], // TODO: Extract implemented traits
            methods: vec![],            // TODO: Extract methods
            fields: vec![],             // TODO: Extract fields
            variants: vec![],           // TODO: Extract variants
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
            .map_err(|e| format!("Failed to generate rustdoc JSON: {}", e))?;

        // Parse the JSON
        let json_str = std::fs::read_to_string(&json_path)
            .map_err(|e| format!("Failed to read JSON file: {}", e))?;

        eprintln!("{}", &json_str[..20.min(json_str.len())]);

        let mut deserializer = serde_json::Deserializer::from_str(&json_str);
        deserializer.disable_recursion_limit();
        let krate = serde::de::Deserialize::deserialize(&mut deserializer)
            .map_err(|e| format!("Failed to parse rustdoc JSON: {}", e))?;

        Ok(krate)
    }
}
