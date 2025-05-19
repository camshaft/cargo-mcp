//! Tests for the MCP server implementation.
//! Each module corresponds to a section in the technical specification.

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

mod caching;
mod configuration;
mod error_handling;
mod general;
mod performance;
mod resources;
mod security;

use crate::{Config, Server};
use rmcp::{
    RoleClient, ServiceError, ServiceExt,
    model::{ReadResourceRequestParam, ReadResourceResult, ResourceContents},
    service::{QuitReason, RunningService},
};
use std::{io, path::PathBuf, sync::Arc};
use tempfile::TempDir;
use tokio::sync::Mutex;

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
    pub fn file(&self, path: &str, content: &str) -> io::Result<PathBuf> {
        let full_path = self.root.join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full_path, content)?;
        Ok(full_path)
    }
}

/// Manages communication with an MCP server
#[derive(Clone)]
pub struct Test {
    /// The client side of the connection
    client: Arc<Mutex<RunningService<RoleClient, ()>>>,
}

impl Test {
    /// Create a new MCP server instance with a duplex connection
    pub async fn start(ctx: Arc<TestContext>) -> io::Result<Self> {
        let (client, stream) = tokio::io::duplex(1 << 17);

        // Create a project with default configuration
        let config = Config::default();
        let server = Server::new(config, ctx.root().clone());

        // Start the server
        tokio::spawn(async move {
            let server = server.serve(stream).await.unwrap();
            server.waiting().await.unwrap();
        });

        // Initialize the client
        let client = ServiceExt::serve((), client).await.unwrap();
        let client = Arc::new(Mutex::new(client));

        Ok(Self { client })
    }

    pub async fn cancel(self) -> io::Result<QuitReason> {
        // Ok(self.client.cancel().await?)
        Ok(QuitReason::Closed)
    }

    pub async fn read_resource(
        &self,
        uri: impl Into<String>,
    ) -> Result<ReadResourceResult, ServiceError> {
        self.client
            .lock()
            .await
            .read_resource(ReadResourceRequestParam { uri: uri.into() })
            .await
    }
}

pub trait ResourceContentsExt {
    fn as_text(&self) -> Option<&str>;
    fn mime_type(&self) -> Option<&str>;
}

impl ResourceContentsExt for ResourceContents {
    fn as_text(&self) -> Option<&str> {
        match self {
            ResourceContents::TextResourceContents { text, .. } => Some(text),
            _ => None,
        }
    }

    fn mime_type(&self) -> Option<&str> {
        match self {
            ResourceContents::TextResourceContents { mime_type, .. } => mime_type.as_deref(),
            _ => None,
        }
    }
}
