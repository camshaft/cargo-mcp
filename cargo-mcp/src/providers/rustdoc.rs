use rustdoc_types::{Id, ItemEnum, ItemKind};
use serde::Serialize;
use std::{
    collections::HashMap,
    ops,
    path::Path,
    sync::{Arc, Mutex},
};

pub const NIGHTLY_VERSION: &str = "nightly-2025-05-09";

type Error = Box<dyn std::error::Error + Send + Sync>;
type Result<T = (), E = Error> = std::result::Result<T, E>;

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub id: Id,
    pub name: String,
    pub path: String,
    pub search: String,
    pub kind: ItemKind,
    pub docs: Option<String>,
    pub children: Vec<Id>,
}

impl Item {
    fn child_path(&self, name: &str) -> (String, String) {
        let path = format!("{}::{name}", self.path);
        let search = if self.search.is_empty() {
            name.to_string()
        } else {
            format!("{}::{name}", self.search)
        };
        (path, search)
    }
}

#[derive(Debug, Clone)]
pub struct Crate {
    pub root_id: Id,
    pub name: String,
    pub items: HashMap<Id, Item>,
    pub paths: HashMap<String, Vec<Id>>, // Maps fully qualified paths to item IDs
}

impl Crate {
    fn from_crate(krate: &rustdoc_types::Crate, crate_name: Option<&str>) -> Self {
        let (crate_id, crate_name) = match crate_name {
            Some(name) => {
                let id = *krate
                    .external_crates
                    .iter()
                    .find(|(_id, info)| info.name == name)
                    .unwrap()
                    .0;

                (id, name.to_string())
            }
            None => {
                let item = krate.index.get(&krate.root).unwrap();
                let id = item.crate_id;
                let name = item.name.clone().unwrap();
                (id, name)
            }
        };

        let mut processed = Self {
            root_id: krate.root,
            name: crate_name,
            items: HashMap::new(),
            paths: HashMap::new(),
        };

        // First pass: Create all items
        for (&id, item) in &krate.paths {
            let Some(info) = krate.index.get(&id) else {
                continue;
            };

            if info.crate_id != crate_id {
                continue;
            }

            let path = item.path.join("::");
            let search = path
                .trim_start_matches(&processed.name)
                .trim_start_matches("::")
                .to_string();

            let kind = item.kind;

            let docs = info.docs.clone();

            let item = Item {
                id,
                name: item.path.last().unwrap().clone(),
                path,
                search,
                kind,
                docs,
                children: Vec::new(),
            };

            processed.items.insert(id, item);
        }

        // Build children relationships
        let mut additional_items = HashMap::new();
        for (&id, item) in &mut processed.items {
            let info = krate.index.get(&id).unwrap();

            if let ItemEnum::Struct(s) = &info.inner {
                // Process impl blocks
                for &impl_id in &s.impls {
                    let Some(impl_info) = krate.index.get(&impl_id) else {
                        continue;
                    };

                    // Process items in the impl block
                    if let ItemEnum::Impl(impl_) = &impl_info.inner {
                        // TODO handle traits differently
                        if impl_.trait_.is_some() {
                            continue;
                        }

                        for &item_id in &impl_.items {
                            let Some(info) = krate.index.get(&item_id) else {
                                continue;
                            };
                            let Some(item_name) = info.name.as_ref() else {
                                continue;
                            };

                            item.children.push(item_id);

                            let (path, search) = item.child_path(item_name);
                            let kind = match &info.inner {
                                ItemEnum::Function(_) => ItemKind::Function,
                                other => {
                                    eprint!("unhandled {other:?}");
                                    continue;
                                }
                            };

                            let fn_item = Item {
                                id: item_id,
                                name: item_name.clone(),
                                path,
                                search,
                                kind,
                                docs: info.docs.clone(),
                                children: Vec::new(),
                            };
                            additional_items.insert(item_id, fn_item);
                        }
                    }
                }
            }
        }
        processed.items.extend(additional_items);

        processed
    }

    pub fn search(&self, query: &str, max_results: Option<usize>) -> Vec<SearchResult<'_>> {
        let mut exact_matches = Vec::new();
        let mut scored_matches = Vec::new();
        let max_results = max_results.unwrap_or(5);

        let query = query
            .trim_start_matches(&self.name)
            .trim_start_matches("::");

        for item in self.items.values() {
            let mut score = strsim::jaro_winkler(&item.search, query);

            // if it's not a perfect match just try the item name instead
            if score < 1.0 {
                score = strsim::jaro_winkler(&item.name, query);
            }

            let res = SearchResult { score, item };

            if res.score == 1.0 {
                exact_matches.push(res);
                scored_matches.clear();
                continue;
            }

            if !exact_matches.is_empty() {
                continue;
            }

            if res.score > 0.2 {
                scored_matches.push(res);
            }
        }

        // Return exact matches if we found any
        if !exact_matches.is_empty() {
            return exact_matches;
        }

        // Sort fuzzy matches by score, return max_results
        scored_matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        scored_matches.truncate(max_results);
        scored_matches
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct SearchResult<'a> {
    pub score: f64,
    pub item: &'a Item,
}

impl ops::Deref for SearchResult<'_> {
    type Target = Item;

    fn deref(&self) -> &Self::Target {
        self.item
    }
}

pub struct RustdocProvider {
    cache: Arc<Mutex<HashMap<String, Arc<Crate>>>>,
}

impl RustdocProvider {
    pub fn new() -> Result<Self> {
        // Install nightly toolchain at initialization
        rustup_toolchain::install(NIGHTLY_VERSION)
            .map_err(|e| format!("Failed to install nightly toolchain: {e}"))?;

        Ok(Self {
            cache: Default::default(),
        })
    }

    pub async fn get_crate_docs(&self, name: &str, version: Option<&str>) -> Result<Arc<Crate>> {
        let version = version.unwrap_or("*");
        let cache_key = format!("{name}:{version}");

        // Check cache first
        if let Some(krate) = self.cache.lock().unwrap().get(&cache_key).cloned() {
            return Ok(krate.clone());
        }

        // Create temporary workspace
        let temp_dir =
            tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {e}"))?;

        // Create Cargo.toml with the crate as a dependency
        let cargo_toml = format!(
            r#"[package]
name = "temp-workspace"
version = "0.1.0"
edition = "2024"

[dependencies]
{name} = {version:?}
"#
        );

        std::fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml)
            .map_err(|e| format!("Failed to write Cargo.toml: {e}"))?;

        // Create empty lib.rs
        std::fs::create_dir_all(temp_dir.path().join("src"))
            .map_err(|e| format!("Failed to create src dir: {e}"))?;
        std::fs::write(temp_dir.path().join("src/lib.rs"), "")
            .map_err(|e| format!("Failed to write lib.rs: {e}"))?;

        // Run cargo vendor to fetch dependencies
        let status = std::process::Command::new("cargo")
            .current_dir(temp_dir.path())
            .arg("vendor")
            .status()
            .map_err(|e| format!("Failed to run cargo vendor: {e}"))?;

        if !status.success() {
            return Err("cargo vendor failed".into());
        }

        // Find the vendored crate directory
        let vendor_dir = temp_dir.path().join("vendor");
        let entries = std::fs::read_dir(&vendor_dir)
            .map_err(|e| format!("Failed to read vendor dir: {e}"))?;

        let crate_dir = entries
            .filter_map(|e| e.ok())
            .find(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false) && e.file_name() == name)
            .ok_or_else(|| format!("Could not find vendored crate {name}"))?
            .path();

        // Generate rustdoc JSON
        let raw_krate = self.generate_rustdoc_json(&crate_dir, false)?;

        // Process and cache
        let krate = Crate::from_crate(&raw_krate, None);
        let krate = Arc::new(krate);

        // Cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(cache_key.clone(), krate.clone());
        }

        Ok(krate)
    }

    pub async fn get_workspace_docs(&self, path: &Path) -> Result<Arc<Crate>> {
        // For workspace docs, always regenerate since they can change frequently
        let raw_krate = self.generate_rustdoc_json(path, true)?;

        // Process and cache both versions
        let krate = Crate::from_crate(&raw_krate, None);
        let krate = Arc::new(krate);

        Ok(krate)
    }

    fn generate_rustdoc_json(
        &self,
        path: &Path,
        is_workspace: bool,
    ) -> Result<rustdoc_types::Crate> {
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
            .map_err(|e| format!("Failed to generate rustdoc JSON: {e}"))?;

        // Parse the JSON
        let json_str = std::fs::read_to_string(&json_path)
            .map_err(|e| format!("Failed to read JSON file: {e}"))?;

        let mut deserializer = serde_json::Deserializer::from_str(&json_str);
        deserializer.disable_recursion_limit();
        let krate = serde::de::Deserialize::deserialize(&mut deserializer)
            .map_err(|e| format!("Failed to parse rustdoc JSON: {e}"))?;

        Ok(krate)
    }
}
