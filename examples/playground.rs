
use std::time::Duration;

use anyhow::Result;
use cargo_mcp::{providers::Providers, Config, Server};
use rmcp::{
    model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation}, transport::{streamable_http_server::session::local::LocalSessionManager, StreamableHttpClientTransport, StreamableHttpService}, ServiceExt
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("info,{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let server_jh = tokio::spawn(bind_server());

    // Await server initialisation
    tokio::time::sleep(Duration::from_secs(1)).await;

    let transport = StreamableHttpClientTransport::from_uri("http://localhost:8000/mcp");
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "test sse client".to_string(),
            version: "0.0.1".to_string(),
        },
    };
    let client = client_info.serve(transport).await.inspect_err(|e| {
        tracing::error!("client error: {:?}", e);
    })?;

    // Initialize
    let server_info = client.peer_info();
    tracing::info!("Connected to server: {server_info:#?}");

    // List tools
    let tools = client.list_tools(Default::default()).await?;
    tracing::info!("Available tools: {tools:#?}");

    let tool_result = client
        .call_tool(CallToolRequestParam {
            name: "search_crate".into(),
            arguments: serde_json::json!({
                "query": "Error sink",
                "crate_name": "bb8"
            }).as_object().cloned(),
        })
        .await?;
    tracing::info!("Tool result: {tool_result:#?}");
    client.cancel().await?;
    server_jh.abort();

    Ok(())
}

const BIND_ADDRESS: &str = "127.0.0.1:8000";

async fn bind_server() -> anyhow::Result<()> {
    let service = StreamableHttpService::new(
        || {
            let config = Config::default();
            eprintln!("Starting cargo-mcp with config: {:#?}", config);

            let providers = Providers::new(&config);
            Ok(Server::new(providers))
        },
        LocalSessionManager::default().into(),
        Default::default(),
    );

    let router = axum::Router::new().nest_service("/mcp", service);
    let tcp_listener = tokio::net::TcpListener::bind(BIND_ADDRESS).await?;
    let _ = axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.unwrap() })
        .await;
    Ok(())
}
