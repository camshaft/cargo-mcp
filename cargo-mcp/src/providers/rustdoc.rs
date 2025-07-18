use rustdoc_types::{Id, ItemEnum, ItemKind};
use serde::Serialize;
use std::{
    collections::HashMap,
    ops,
    path::Path,
    sync::{Arc, Mutex},
};

pub const NIGHTLY_VERSION: &str = "nightly-2025-07-16";

type Error = Box<dyn std::error::Error + Send + Sync>;
type Result<T = (), E = Error> = std::result::Result<T, E>;

type Str = Arc<str>;

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    #[serde(skip)]
    pub id: Id,
    #[serde(skip)] // just return the full path
    pub name: Str,
    pub path: Str,
    #[serde(skip)] // this is just used for easier searching
    pub search: Str,
    pub kind: ItemKind,
    #[serde(skip)]
    pub docs: Option<Str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub functions: Vec<Str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<Str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<Str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trait_impls: Vec<Str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub structs: Vec<Str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub enums: Vec<Str>,
}

impl Item {
    fn new(id: Id, name: Str, path: Str, search: Str, kind: ItemKind, docs: Option<Str>) -> Self {
        Self {
            id,
            name,
            path,
            search,
            kind,
            docs,
            functions: vec![],
            variants: vec![],
            traits: vec![],
            trait_impls: vec![],
            structs: vec![],
            enums: vec![],
        }
    }

    fn from_summary(
        id: Id,
        summary: &rustdoc_types::ItemSummary,
        item: &rustdoc_types::Item,
        krate: &Crate,
    ) -> Self {
        let path = Str::from(summary.path.join("::"));
        let search = path
            .trim_start_matches(&*krate.name)
            .trim_start_matches("::")
            .to_string()
            .into();

        let kind = summary.kind;

        let docs = item.docs.clone().map(Str::from);

        let name = summary.path.last().unwrap().clone().into();

        Self::new(id, name, path, search, kind, docs)
    }

    fn sort(&mut self) {
        let lists = [
            &mut self.functions,
            &mut self.variants,
            &mut self.traits,
            &mut self.trait_impls,
            &mut self.structs,
            &mut self.enums,
        ];

        for list in lists {
            list.sort();
            list.dedup();
        }
    }

    fn child_path(&self, name: &str) -> (Str, Str) {
        let path = format!("{}::{name}", self.path);
        let search = if self.search.is_empty() {
            name.to_string()
        } else {
            format!("{}::{name}", self.search)
        };
        (path.into(), search.into())
    }

    fn resolve_children(
        &mut self,
        info: &rustdoc_types::Item,
        krate: &rustdoc_types::Crate,
        additional_items: &mut HashMap<Id, Item>,
    ) {
        match &info.inner {
            ItemEnum::Struct(s) => {
                // Process impl blocks for structs
                for &impl_id in &s.impls {
                    let Some(impl_info) = krate.index.get(&impl_id) else {
                        continue;
                    };

                    // Process items in the impl block
                    if let ItemEnum::Impl(impl_) = &impl_info.inner {
                        if let Some(i) = impl_.trait_.as_ref() {
                            self.push_trait_impl(i);
                            continue;
                        }

                        for &item_id in &impl_.items {
                            let Some(info) = krate.index.get(&item_id) else {
                                continue;
                            };
                            let Some(item_name) = info.name.as_ref() else {
                                continue;
                            };

                            self.functions.push(item_name.clone().into());

                            let (path, search) = self.child_path(item_name);
                            let kind = match &info.inner {
                                ItemEnum::Function(_) => ItemKind::Function,
                                other => {
                                    eprint!("unhandled struct impl item {other:?}");
                                    continue;
                                }
                            };

                            let fn_item = Item::new(
                                item_id,
                                item_name.clone().into(),
                                path,
                                search,
                                kind,
                                info.docs.clone().map(Str::from),
                            );
                            additional_items.insert(item_id, fn_item);
                        }
                    }
                }
            }
            ItemEnum::Enum(e) => {
                // Process enum variants
                for variant in &e.variants {
                    let Some(variant_info) = krate.index.get(variant) else {
                        continue;
                    };
                    let Some(variant_name) = variant_info.name.as_ref() else {
                        continue;
                    };

                    self.variants.push(variant_name.clone().into());

                    let (path, search) = self.child_path(variant_name);
                    let kind = ItemKind::Variant;

                    let variant_item = Item::new(
                        *variant,
                        variant_name.clone().into(),
                        path,
                        search,
                        kind,
                        variant_info.docs.clone().map(Str::from),
                    );
                    additional_items.insert(*variant, variant_item);
                }

                // Process impl blocks for enums
                for &impl_id in &e.impls {
                    let Some(impl_info) = krate.index.get(&impl_id) else {
                        continue;
                    };

                    // Process items in the impl block
                    if let ItemEnum::Impl(impl_) = &impl_info.inner {
                        if let Some(i) = impl_.trait_.as_ref() {
                            self.push_trait_impl(i);
                            continue;
                        }

                        for &item_id in &impl_.items {
                            let Some(info) = krate.index.get(&item_id) else {
                                continue;
                            };
                            let Some(item_name) = info.name.as_ref() else {
                                continue;
                            };

                            self.functions.push(item_name.clone().into());

                            let (path, search) = self.child_path(item_name);
                            let kind = match &info.inner {
                                ItemEnum::Function(_) => ItemKind::Function,
                                other => {
                                    eprint!("unhandled enum impl item {other:?}");
                                    continue;
                                }
                            };

                            let fn_item = Item::new(
                                item_id,
                                item_name.clone().into(),
                                path,
                                search,
                                kind,
                                info.docs.clone().map(Str::from),
                            );
                            additional_items.insert(item_id, fn_item);
                        }
                    }
                }
            }
            _ => {
                // Handle other types if needed in the future
            }
        }
    }

    fn push_trait_impl(&mut self, path: &rustdoc_types::Path) {
        let name = path.path.to_string();
        if let Some(args) = &path.args {
            use rustdoc_types::GenericArgs;

            match &**args {
                GenericArgs::AngleBracketed { args, constraints } => {
                    // TODO
                    let _ = args;
                    let _ = constraints;
                }
                GenericArgs::Parenthesized { inputs, output } => {
                    // TODO
                    let _ = inputs;
                    let _ = output;
                }
                GenericArgs::ReturnTypeNotation => {
                    // TODO
                }
            }
        }
        self.trait_impls.push(name.into());
    }
}

#[derive(Debug, Clone)]
pub struct Crate {
    pub root_id: Id,
    pub name: Str,
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
            name: crate_name.into(),
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

            let item = Item::from_summary(id, item, &info, &processed);

            processed.items.insert(id, item);
        }

        // Build children relationships
        let mut additional_items = HashMap::new();
        for (&id, item) in &mut processed.items {
            let info = krate.index.get(&id).unwrap();
            item.resolve_children(info, krate, &mut additional_items);
            item.sort();
        }
        processed.items.extend(additional_items);

        processed
    }

    pub fn search(&self, query: &str, max_results: Option<usize>) -> Vec<SearchResult<'_>> {
        let mut exact_matches = Vec::new();
        let mut scored_matches = Vec::new();
        let max_results = max_results.unwrap_or(5);

        let query = query
            .trim_start_matches(&*self.name)
            .trim_start_matches("::");

        for item in self.items.values() {
            let mut score = strsim::jaro_winkler(&item.search, query);

            // if it's not a perfect match just try the item name instead
            if score < 1.0 {
                score = score.max(strsim::jaro_winkler(&item.name, query));
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
            .stdout(std::process::Stdio::null())
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

        match deserialize_str(&json_str) {
            Ok(value) => Ok(value),
            Err(err) => {
                // try to parse as a json value to check the format version
                let Ok(value) = deserialize_str::<serde_json::Value>(&json_str) else {
                    return Err(err);
                };

                if let Some(format_version) = value.get("format_version").and_then(|v| v.as_u64()) {
                    if rustdoc_types::FORMAT_VERSION as u64 != format_version {
                        return Err(format!(
                            "rustdoc JSON format version mismatch: expected {}, got {format_version}",
                            rustdoc_types::FORMAT_VERSION,
                        )
                        .into());
                    }
                }

                Err(err)
            }
        }
    }
}

fn deserialize_str<T: serde::de::DeserializeOwned>(v: &str) -> Result<T> {
    let mut deserializer = serde_json::Deserializer::from_str(v);
    deserializer.disable_recursion_limit();

    let value = serde::de::Deserialize::deserialize(&mut deserializer)
        .map_err(|e| format!("Failed to parse rustdoc JSON: {e}"))?;

    Ok(value)
}
