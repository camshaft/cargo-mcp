//! Tests for the MCP server implementation.
//! Each module corresponds to a section in the technical specification.

use crate::{Config, Server, providers::Providers};
use rmcp::{
    RoleClient, ServiceError, ServiceExt,
    model::{CallToolRequestParam, CallToolResult},
    service::{QuitReason, RunningService},
};
use serde_json::Value;
use std::{io, path::PathBuf, sync::Arc};
use tempfile::TempDir;
use tokio::sync::Mutex;

#[macro_export]
macro_rules! assert_json_matches {
    ($actual:expr, $pattern:expr) => {
        let actual = &$actual;
        let pattern = &$pattern;

        // TODO: Implement pattern matching
        // For now just ensure actual matches pattern exactly
        assert_eq!(actual, pattern);
    };
}

mod general;
mod rustdoc;
mod tools;

/// Manages a temporary test environment
pub struct TestContext {
    /// Root directory for this test
    root: PathBuf,
    /// Temporary directory that will be cleaned up
    _temp_dir: TempDir,
}

impl TestContext {
    /// Create a new test context with a temporary directory
    pub fn new() -> io::Result<Self> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path().into();

        Ok(Self {
            root,
            _temp_dir: temp_dir,
        })
    }

    /// Get the root directory path
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Create a file with given content
    pub fn file(&self, path: &str, content: &str) -> PathBuf {
        let full_path = self.root.join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&full_path, content).unwrap();
        full_path
    }
}

/// Manages communication with an MCP server
#[derive(Clone)]
pub struct Test {
    /// The client side of the connection
    client: Arc<Mutex<RunningService<RoleClient, ()>>>,
    #[allow(dead_code)]
    ctx: Arc<TestContext>,
}

impl Test {
    /// Create a new MCP server instance with a duplex connection
    pub async fn start(ctx: Arc<TestContext>) -> io::Result<Self> {
        let (client, stream) = tokio::io::duplex(1 << 17);

        // Create a project with default configuration
        let mut config = Config::default();
        config.pwd = ctx.root.clone().into();
        let providers = Providers::new(&config);
        let server = Server::new(providers);

        // Start the server
        tokio::spawn(async move {
            let server = server.serve(stream).await.unwrap();
            server.waiting().await.unwrap();
        });

        // Initialize the client
        let client = ServiceExt::serve((), client).await.unwrap();
        let client = Arc::new(Mutex::new(client));

        Ok(Self { client, ctx })
    }

    pub async fn cancel(self) -> io::Result<QuitReason> {
        // Ok(self.client.cancel().await?)
        Ok(QuitReason::Closed)
    }

    // pub async fn read_resource(
    //     &self,
    //     uri: impl Into<String>,
    // ) -> Result<ReadResourceResult, ServiceError> {
    //     self.client
    //         .lock()
    //         .await
    //         .read_resource(ReadResourceRequestParam { uri: uri.into() })
    //         .await
    // }

    pub async fn call_tool(
        &self,
        tool_name: impl Into<String>,
        params: impl IntoIterator<Item = (impl Into<String>, impl Into<Value>)>,
    ) -> Result<CallToolResult, ServiceError> {
        let arguments: serde_json::Map<_, _> = params
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        let arguments = if arguments.is_empty() {
            None
        } else {
            Some(arguments)
        };

        self.client
            .lock()
            .await
            .call_tool(CallToolRequestParam {
                name: tool_name.into().into(),
                arguments,
            })
            .await
    }
}
