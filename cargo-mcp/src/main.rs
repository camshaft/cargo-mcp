use cargo_mcp::{Config, Server, providers::Providers};
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    eprintln!("Starting cargo-mcp with config: {config:#?}");

    let providers = Providers::new(&config);
    let service = Server::new(providers).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
