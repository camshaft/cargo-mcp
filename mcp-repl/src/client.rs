use anyhow::Result;
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation, Tool},
    service::ServerSink,
    transport::{StreamableHttpClientTransport, TokioChildProcess},
};
use serde_json::Value;
use std::{collections::HashMap, ops};
use tokio::process::Command;

/// Transport type for MCP connections
#[derive(Debug, Clone)]
pub enum Transport {
    /// Connect to a server via stdio (subprocess)
    Stdio { command: String, args: Vec<String> },
    /// Connect to a server via HTTP
    Http { url: String },
}

type Inner = Box<dyn ops::Deref<Target = ServerSink>>;

/// MCP Client for interacting with MCP servers
pub struct McpClient {
    client: Inner,
    tools: HashMap<String, Tool>,
}

impl McpClient {
    /// Create a new MCP client with the specified transport
    pub async fn new(transport: Transport) -> Result<Self> {
        let client: Inner = match transport {
            Transport::Stdio { command, args } => {
                let mut cmd = Command::new(&command);
                for arg in args {
                    cmd.arg(arg);
                }

                let client_info = ClientInfo {
                    protocol_version: Default::default(),
                    capabilities: ClientCapabilities::default(),
                    client_info: Implementation {
                        name: "mcp-repl".to_string(),
                        version: "0.1.0".to_string(),
                    },
                };

                let handle = client_info.serve(TokioChildProcess::new(cmd)?).await?;
                Box::new(handle)
            }
            Transport::Http { url } => {
                let transport = StreamableHttpClientTransport::from_uri(url);
                let client_info = ClientInfo {
                    protocol_version: Default::default(),
                    capabilities: ClientCapabilities::default(),
                    client_info: Implementation {
                        name: "mcp-repl".to_string(),
                        version: "0.1.0".to_string(),
                    },
                };

                let handle = client_info.serve(transport).await?;
                Box::new(handle)
            }
        };

        let mut mcp_client = Self {
            client,
            tools: HashMap::new(),
        };

        // Load available tools
        mcp_client.refresh_tools().await?;

        Ok(mcp_client)
    }

    /// Get server information
    pub fn server_info(&self) -> &rmcp::model::ServerInfo {
        self.client.peer_info().unwrap()
    }

    /// Refresh the list of available tools from the server
    pub async fn refresh_tools(&mut self) -> Result<()> {
        let tools = self.client.list_all_tools().await?;

        self.tools.clear();
        for tool in tools {
            self.tools.insert(tool.name.to_string(), tool);
        }

        Ok(())
    }

    /// Get the list of available tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get information about a specific tool
    pub fn get_tool(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    /// Call a tool with the given arguments
    pub async fn call_tool(&self, name: &str, arguments: Option<Value>) -> Result<Value> {
        if !self.tools.contains_key(name) {
            return Err(anyhow::anyhow!("Tool '{}' not found", name));
        }

        let result = self
            .client
            .call_tool(CallToolRequestParam {
                name: name.to_string().into(),
                arguments: arguments.and_then(|v| v.as_object().cloned()),
            })
            .await?;

        Ok(serde_json::to_value(result)?)
    }
}
