use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Documentation response structure containing API information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Documentation {
    pub public_api: Vec<ApiItem>,
    pub modules: Vec<Module>,
    pub types: Vec<Type>,
    pub traits: Vec<Trait>,
}

/// API item representing a single documented item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiItem {
    pub name: String,
    pub kind: ApiItemKind,
    pub visibility: Visibility,
    pub documentation: Option<String>,
    pub attributes: Vec<Attribute>,
}

/// Kind of API item (function, type, trait, etc)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiItemKind {
    Function,
    Type,
    Trait,
    Module,
    Constant,
    Macro,
}

/// Visibility level of an item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Crate,
    Restricted(String),
}

/// Module information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub path: String,
    pub documentation: Option<String>,
    pub items: Vec<String>,
}

/// Type definition information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Type {
    pub name: String,
    pub documentation: Option<String>,
    pub kind: TypeKind,
    pub attributes: Vec<Attribute>,
}

/// Kind of type (struct, enum, etc)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeKind {
    Struct,
    Enum,
    Union,
    TypeAlias,
}

/// Trait definition information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trait {
    pub name: String,
    pub documentation: Option<String>,
    pub items: Vec<String>,
    pub attributes: Vec<Attribute>,
}

/// Attribute attached to an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub args: Option<String>,
}

/// Crate information response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateInfo {
    pub latest_version: String,
    pub all_versions: Vec<String>,
    pub features: HashMap<String, Vec<String>>,
    pub dependencies: Vec<Dependency>,
    pub description: Option<String>,
    pub repository: Option<String>,
}

/// Dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version_req: String,
    pub features: Vec<String>,
    pub optional: bool,
}

/// Project metadata response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub workspace_members: Vec<Package>,
    pub dependencies: Vec<Dependency>,
    pub targets: Vec<Target>,
    pub features: HashMap<String, Vec<String>>,
}

/// Package information in a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub path: String,
}

/// Build target information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub name: String,
    pub kind: Vec<String>,
    pub crate_types: Vec<String>,
    pub required_features: Vec<String>,
}

impl Documentation {
    pub fn from_public_api(api: public_api::PublicApi) -> Self {
        let mut items = Vec::new();
        let mut modules = Vec::new();
        let mut types = Vec::new();
        let mut traits = Vec::new();

        // Convert each public API item
        for item in api.items() {
            let item_str = item.to_string();
            let tokens = item.tokens().collect::<Vec<_>>();

            // Extract basic information
            let name = item_str.lines().next().unwrap_or_default().to_string();
            let documentation = tokens
                .iter()
                .filter(|t| matches!(t, public_api::tokens::Token::Annotation(_)))
                .map(|t| t.text().to_string())
                .collect::<Vec<_>>()
                .join("\n");

            // Extract attributes
            let attributes = tokens
                .iter()
                .filter_map(|t| match t {
                    public_api::tokens::Token::Annotation(a) if !a.starts_with("///") => {
                        let parts: Vec<_> = a.splitn(2, '(').collect();
                        Some(Attribute {
                            name: parts[0]
                                .trim_start_matches('#')
                                .trim_start_matches('[')
                                .to_string(),
                            args: parts
                                .get(1)
                                .map(|s| s.trim_end_matches(']').trim_end_matches(')').to_string()),
                        })
                    }
                    _ => None,
                })
                .collect();

            // Determine item kind and categorize
            let mut api_item = ApiItem {
                name: name.clone(),
                kind: ApiItemKind::Function, // Default, will be updated
                visibility: Visibility::Public, // Default, will be updated
                documentation: if documentation.is_empty() {
                    None
                } else {
                    Some(documentation)
                },
                attributes,
            };

            // Update kind and categorize
            for token in tokens {
                match token {
                    public_api::tokens::Token::Kind(k) => {
                        api_item.kind = match k.as_str() {
                            "fn" => ApiItemKind::Function,
                            "type" => {
                                types.push(Type {
                                    name: name.clone(),
                                    documentation: api_item.documentation.clone(),
                                    kind: TypeKind::TypeAlias,
                                    attributes: api_item.attributes.clone(),
                                });
                                ApiItemKind::Type
                            }
                            "trait" => {
                                traits.push(Trait {
                                    name: name.clone(),
                                    documentation: api_item.documentation.clone(),
                                    items: Vec::new(), // Will be populated later
                                    attributes: api_item.attributes.clone(),
                                });
                                ApiItemKind::Trait
                            }
                            "mod" => {
                                modules.push(Module {
                                    name: name.clone(),
                                    path: name.clone(),
                                    documentation: api_item.documentation.clone(),
                                    items: Vec::new(), // Will be populated later
                                });
                                ApiItemKind::Module
                            }
                            "const" => ApiItemKind::Constant,
                            "macro" => ApiItemKind::Macro,
                            _ => continue,
                        };
                    }
                    public_api::tokens::Token::Qualifier(q) => {
                        api_item.visibility = match q.as_str() {
                            "pub" => Visibility::Public,
                            "pub(crate)" => Visibility::Crate,
                            q if q.starts_with("pub(in ") => Visibility::Restricted(
                                q.trim_start_matches("pub(in ")
                                    .trim_end_matches(')')
                                    .to_string(),
                            ),
                            _ => Visibility::Public,
                        };
                    }
                    _ => {}
                }
            }

            items.push(api_item);
        }

        Self {
            public_api: items,
            modules,
            types,
            traits,
        }
    }
}

impl ProjectMetadata {
    pub fn from_cargo_metadata(metadata: cargo_metadata::Metadata) -> Self {
        Self {
            workspace_members: metadata
                .workspace_members
                .iter()
                .filter_map(|id| {
                    metadata
                        .packages
                        .iter()
                        .find(|p| p.id == *id)
                        .map(|p| Package {
                            name: p.name.clone(),
                            version: p.version.to_string(),
                            path: p.manifest_path.as_std_path().to_string_lossy().into_owned(),
                        })
                })
                .collect(),
            dependencies: metadata
                .resolve
                .as_ref()
                .map(|resolve| {
                    resolve
                        .nodes
                        .iter()
                        .map(|node| {
                            let pkg = metadata
                                .packages
                                .iter()
                                .find(|p| p.id == node.id)
                                .expect("package not found");
                            Dependency {
                                name: pkg.name.clone(),
                                version_req: pkg.version.to_string(),
                                features: node.features.iter().cloned().collect(),
                                optional: pkg.dependencies.iter().any(|d| d.optional),
                            }
                        })
                        .collect()
                })
                .unwrap_or_default(),
            targets: metadata
                .packages
                .iter()
                .flat_map(|p| {
                    p.targets.iter().map(|t| Target {
                        name: t.name.clone(),
                        kind: t.kind.iter().map(|v| v.to_string()).collect(),
                        crate_types: t.crate_types.iter().map(|v| v.to_string()).collect(),
                        required_features: t.required_features.clone(),
                    })
                })
                .collect(),
            features: metadata.packages.iter().fold(HashMap::new(), |mut acc, p| {
                acc.extend(p.features.clone());
                acc
            }),
        }
    }
}

impl CrateInfo {
    pub fn from_cargo_output(output: &[u8]) -> crate::Result<Self> {
        let output_str = String::from_utf8_lossy(output);
        let mut lines = output_str.lines();

        let mut latest_version = String::new();
        let mut all_versions = Vec::new();
        let mut features = HashMap::new();
        let mut dependencies = Vec::new();
        let mut description = None;
        let mut repository = None;

        while let Some(line) = lines.next() {
            if line.starts_with("Version: ") {
                latest_version = line.trim_start_matches("Version: ").to_string();
                all_versions.push(latest_version.clone());
            } else if line.starts_with("All versions: ") {
                all_versions = line
                    .trim_start_matches("All versions: ")
                    .split(", ")
                    .map(|s| s.to_string())
                    .collect();
            } else if line.starts_with("Features: ") {
                // Features are listed one per line after this
                while let Some(feature_line) = lines.next() {
                    if feature_line.is_empty() {
                        break;
                    }
                    let parts: Vec<_> = feature_line.splitn(2, " - ").collect();
                    if parts.len() == 2 {
                        let name = parts[0].trim().to_string();
                        let deps = parts[1].split(',').map(|s| s.trim().to_string()).collect();
                        features.insert(name, deps);
                    }
                }
            } else if line.starts_with("Dependencies: ") {
                // Dependencies are listed one per line after this
                while let Some(dep_line) = lines.next() {
                    if dep_line.is_empty() {
                        break;
                    }
                    let parts: Vec<_> = dep_line.splitn(3, " ").collect();
                    if parts.len() >= 2 {
                        dependencies.push(Dependency {
                            name: parts[0].to_string(),
                            version_req: parts[1].to_string(),
                            features: Vec::new(), // Not available in cargo info output
                            optional: parts.get(2).map_or(false, |s| *s == "(optional)"),
                        });
                    }
                }
            } else if line.starts_with("Description: ") {
                description = Some(line.trim_start_matches("Description: ").to_string());
            } else if line.starts_with("Repository: ") {
                repository = Some(line.trim_start_matches("Repository: ").to_string());
            }
        }

        Ok(Self {
            latest_version,
            all_versions,
            features,
            dependencies,
            description,
            repository,
        })
    }
}
