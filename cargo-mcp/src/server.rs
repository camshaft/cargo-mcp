use crate::providers::Providers;
use cargo_metadata::Metadata;
use rmcp::{
    handler::server::{
        ServerHandler,
        tool::{Parameters, ToolRouter},
    },
    model::{
        CallToolResult, Content, ErrorData, Implementation, ProtocolVersion, ServerCapabilities,
        ServerInfo,
    },
    tool, tool_handler, tool_router,
};
use serde_json::json;
use std::sync::Arc;

type McpResult<T = (), E = rmcp::ErrorData> = core::result::Result<T, E>;

#[derive(Clone)]
pub struct Server {
    state: Arc<Providers>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct WorkspaceCrates {
    directory: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct WorkspaceCrateInfo {
    #[schemars(description = "An absolute path to the directory the crate resides")]
    directory: String,
    #[schemars(description = "The name of the crate")]
    crate_name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct CratesIoLatestVersion {
    #[schemars(description = "The name of the crate")]
    crate_name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct CratesIoVersions {
    #[schemars(description = "The name of the crate")]
    crate_name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct CratesIoFeatures {
    #[schemars(description = "The name of the crate")]
    crate_name: String,
    #[schemars(
        description = "Optional version to get features for. If not provided, uses latest version."
    )]
    version: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SearchCrate {
    #[schemars(description = "The item to search for")]
    query: String,
    #[schemars(description = "The name of the crate")]
    crate_name: String,
    #[schemars(
        description = "Optional version to get features for. If not provided, uses latest version."
    )]
    version: Option<String>,
    #[schemars(description = "Max results to return")]
    max_results: Option<usize>,
}

#[tool_router]
impl Server {
    pub fn new(providers: Providers) -> Self {
        Self {
            state: Arc::new(providers),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List all of the workspace member crates")]
    async fn workspace_crates(
        &self,
        Parameters(params): Parameters<WorkspaceCrates>,
    ) -> McpResult<CallToolResult> {
        let directory = params.directory;
        let meta = self.metadata(&directory)?;

        let mut crates = vec![];

        for package in meta.workspace_packages() {
            let directory = &package.manifest_path;
            let directory = directory.parent().unwrap_or(directory);
            let directory = format!("file://{directory}");
            crates.push(json!({
                "name": &package.name,
                "directory": directory,
            }))
        }

        Ok(CallToolResult::success(vec![
            Content::json(crates).unwrap(),
        ]))
    }

    #[tool(description = "Returns information about a specific crate")]
    async fn workspace_crate_info(
        &self,
        Parameters(params): Parameters<WorkspaceCrateInfo>,
    ) -> McpResult<CallToolResult> {
        let directory = params.directory;
        let crate_name = params.crate_name;
        let meta = self.metadata(&directory)?;

        let packages = meta.workspace_packages();
        let package = packages.iter().find(|pkg| pkg.name == crate_name);

        let Some(package) = package else {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Could not find crate {crate_name:?}"
            ))]));
        };

        Ok(CallToolResult::success(vec![
            Content::json(package).unwrap(),
        ]))
    }

    #[tool(description = "Returns the latest version for a given crate from crates.io")]
    async fn crates_io_latest_version(
        &self,
        Parameters(params): Parameters<CratesIoLatestVersion>,
    ) -> McpResult<CallToolResult> {
        let crate_name = params.crate_name;
        match self.state.crates_io.fetch_latest_version(&crate_name).await {
            Ok(version) => Ok(CallToolResult::success(vec![
                Content::json(json!({
                    "name": crate_name,
                    "version": version
                }))
                .unwrap(),
            ])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get latest version for crate {crate_name}: {err}"
            ))])),
        }
    }

    #[tool(description = "Returns all available versions for a given crate from crates.io")]
    async fn crates_io_versions(
        &self,
        Parameters(params): Parameters<CratesIoVersions>,
    ) -> McpResult<CallToolResult> {
        let crate_name = params.crate_name;
        match self.state.crates_io.fetch_versions(&crate_name).await {
            Ok(versions) => {
                let versions: Vec<_> = versions
                    .iter()
                    .map(|v| {
                        json!({
                            "version": v.version(),
                            "yanked": v.is_yanked(),
                            "msrv": v.rust_version(),
                        })
                    })
                    .collect();
                Ok(CallToolResult::success(vec![
                    Content::json(json!({
                        "name": crate_name,
                        "versions": versions
                    }))
                    .unwrap(),
                ]))
            }
            Err(err) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get versions for crate {crate_name}: {err}"
            ))])),
        }
    }

    #[tool(description = "Returns the list of features for a given crate from crates.io")]
    async fn crates_io_features(
        &self,
        Parameters(params): Parameters<CratesIoFeatures>,
    ) -> McpResult<CallToolResult> {
        let crate_name = params.crate_name;
        let version = params.version;
        match self
            .state
            .crates_io
            .fetch_features(&crate_name, version.as_deref())
            .await
        {
            Ok(features) => Ok(CallToolResult::success(vec![
                Content::json(json!({
                    "name": crate_name,
                    "version": version.unwrap_or_else(|| "latest".to_string()),
                    "features": features
                }))
                .unwrap(),
            ])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get features for crate {crate_name}: {err}",
            ))])),
        }
    }

    #[tool(description = "Search a crate for an item")]
    async fn search_crate(
        &self,
        Parameters(params): Parameters<SearchCrate>,
    ) -> McpResult<CallToolResult> {
        let crate_name = params.crate_name;
        let version = params.version;
        let query = params.query;
        let max_results = params.max_results;

        let krate = match self
            .state
            .rustdoc
            .get_crate_docs(&crate_name, version.as_deref())
            .await
        {
            Ok(krate) => krate,
            Err(err) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to search for crate {crate_name}: {err}",
                ))]));
            }
        };
        let results = krate.search(&query, max_results);
        Ok(CallToolResult::success(vec![
            Content::json(results).unwrap(),
        ]))
    }
}

impl Server {
    fn metadata(&self, dir: &str) -> McpResult<Metadata> {
        let dir = dir.trim_start_matches("file://");
        eprintln!("Server metadata dir: {dir}");
        let meta = self
            .state
            .metadata
            .get_metadata(dir)
            .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;

        Ok(meta)
    }
}

#[tool_handler]
impl ServerHandler for Server {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(include_str!("./instructions.md").into()),
        }
    }
}
