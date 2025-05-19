use crate::{Cache, Config, Error, Result};
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    ListResourcesResult, RawResource, ReadResourceRequestParam, ReadResourceResult, Resource,
    ResourceContents,
};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

type McpResult<T = (), E = rmcp::Error> = core::result::Result<T, E>;

//= docs/design/technical-spec.md#general
//# The server MUST implement the Model Context Protocol (MCP) specification.
pub struct Server {
    state: Arc<State>,
}

struct State {
    dir: PathBuf,
    doc_cache: Cache<Value>,
    info_cache: Cache<Value>,
}

impl Server {
    pub fn new(config: Config, dir: PathBuf) -> Self {
        //= docs/design/technical-spec.md#general
        //# The server MUST support caching of documentation and crate information.
        let doc_cache = Cache::new(config.doc_cache_ttl, config.max_cache_size);
        let info_cache = Cache::new(config.info_cache_ttl, config.max_cache_size);
        let state = State {
            dir,
            doc_cache,
            info_cache,
        };
        Self {
            state: Arc::new(state),
        }
    }

    //= docs/design/technical-spec.md#crate-documentation-resource
    //# The server MUST provide a resource at path `crate/{name}/docs`.
    //# The server MUST accept an optional version parameter in the query string.
    //# The server MUST generate rustdoc JSON using the nightly toolchain.
    //# The server MUST parse the rustdoc JSON using the public-api crate.
    //# The server MUST return documentation in a structured format containing public API items, modules, types, and traits.
    //# The server SHOULD cache documentation results to improve performance.
    async fn get_docs(&self, name: &str, version: Option<&str>) -> Result<Value> {
        // Try cache first
        let cache_key = match version {
            Some(v) => format!("{}-{}", name, v),
            None => name.to_string(),
        };

        if let Some(cached) = self.state.doc_cache.get(&cache_key).await {
            return Ok(cached);
        }

        // Generate rustdoc JSON using nightly toolchain
        let json_path = self.generate_rustdoc_json(name, version).await?;

        // Parse using public-api
        let api = public_api::Builder::from_rustdoc_json(json_path).build()?;

        // Convert to our format
        let docs = crate::Documentation::from_public_api(api);
        let value = serde_json::to_value(docs)?;

        // Cache the result
        self.state.doc_cache.insert(cache_key, value.clone()).await;

        Ok(value)
    }

    //= docs/design/technical-spec.md#crate-information-resource
    //# The server MUST provide a resource at path `crate/{name}/info`.
    //# The server MUST execute the `cargo info` command to retrieve crate information.
    //# The server MUST parse and return the latest version of the crate.
    //# The server MUST return all available versions of the crate.
    //# The server MUST return all available features and their descriptions.
    //# The server MUST return the crate's dependencies.
    //# The server SHOULD cache crate information with a shorter TTL than documentation.
    async fn get_info(&self, name: &str) -> Result<Value> {
        // Try cache first
        if let Some(cached) = self.state.info_cache.get(name).await {
            return Ok(cached);
        }

        // Run cargo command
        let output = tokio::process::Command::new("cargo")
            .current_dir(&self.state.dir)
            .arg("info")
            .arg(name)
            .output()
            .await?;

        if !output.status.success() {
            return Err(Error::CrateNotFound(name.to_string()));
        }

        // Parse output
        let info = crate::CrateInfo::from_cargo_output(&output.stdout)?;
        let value = serde_json::to_value(info)?;

        // Cache the result
        self.state
            .info_cache
            .insert(name.to_string(), value.clone())
            .await;

        Ok(value)
    }

    //= docs/design/technical-spec.md#project-metadata-resource
    //# The server MUST provide a resource at path `project/metadata`.
    //# The server MUST execute the `cargo metadata` command with format version 1.
    //# The server MUST return workspace member information.
    //# The server MUST return dependency information.
    //# The server MUST return target information.
    //# The server MUST return feature information.
    //# The server SHOULD NOT cache project metadata as it should reflect the current state.
    async fn get_metadata(&self) -> Result<Value> {
        let metadata = cargo_metadata::MetadataCommand::new()
            .current_dir(&self.state.dir)
            .exec()?;

        let project_metadata = crate::ProjectMetadata::from_cargo_metadata(metadata);
        serde_json::to_value(project_metadata).map_err(Error::from)
    }

    //= docs/design/technical-spec.md#security-considerations
    //# The server MUST validate all input parameters to prevent command injection.
    //# The server MUST handle file paths securely to prevent path traversal attacks.
    fn validate_crate_name(name: &str) -> Result<()> {
        // Only allow alphanumeric characters, hyphens and underscores
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Error::InvalidVersion(format!(
                "Invalid crate name: {}",
                name
            )));
        }
        Ok(())
    }

    async fn generate_rustdoc_json(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<std::path::PathBuf> {
        Self::validate_crate_name(name)?;

        rustup_toolchain::install(public_api::MINIMUM_NIGHTLY_RUST_VERSION).unwrap();

        // TODO is version is specified then we'll need to create a temporary crate to download it
        let _ = version;

        // Use rustdoc-json to generate JSON
        let json_path = rustdoc_json::Builder::default()
            .toolchain(public_api::MINIMUM_NIGHTLY_RUST_VERSION)
            .manifest_path(self.state.dir.join("Cargo.toml"))
            .build()?;

        Ok(json_path)
    }
}

impl ServerHandler for Server {
    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> McpResult<ListResourcesResult> {
        Ok(ListResourcesResult {
            resources: vec![
                Resource {
                    raw: RawResource {
                        uri: "cargo://crate/{name}/docs".into(),
                        name: "Crate Documentation".into(),
                        description: Some("Get documentation for a crate".into()),
                        mime_type: Some("application/json".into()),
                        size: None,
                    },
                    annotations: None,
                },
                Resource {
                    raw: RawResource {
                        uri: "cargo://crate/{name}/info".into(),
                        name: "Crate Information".into(),
                        description: Some("Get information about a crate".into()),
                        mime_type: Some("application/json".into()),
                        size: None,
                    },
                    annotations: None,
                },
                Resource {
                    raw: RawResource {
                        uri: "cargo://project/metadata".into(),
                        name: "Project Metadata".into(),
                        description: Some("Get metadata about the current project".into()),
                        mime_type: Some("application/json".into()),
                        size: None,
                    },
                    annotations: None,
                },
            ],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> McpResult<ReadResourceResult> {
        let components = request
            .uri
            .as_str()
            .trim_start_matches("cargo://")
            .split('/')
            .collect::<Vec<_>>();

        let result = match &components[..] {
            ["crate", name, "docs"] => {
                let query_str = request.uri.split('?').nth(1).unwrap_or("");
                let pairs: Vec<_> = url::form_urlencoded::parse(query_str.as_bytes()).collect();
                let version = pairs
                    .iter()
                    .find(|(k, _)| k == "version")
                    .map(|(_, v)| v.as_ref());
                self.get_docs(name, version).await?
            }
            ["crate", name, "info"] => self.get_info(name).await?,
            ["project", "metadata"] => self.get_metadata().await?,
            _ => return Err(Error::InvalidVersion("Invalid resource path".into()).into()),
        };

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::TextResourceContents {
                uri: request.uri,
                mime_type: Some("application/json".into()),
                text: serde_json::to_string_pretty(&result).map_err(|e| Error::ParseError(e))?,
            }],
        })
    }
}
