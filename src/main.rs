use cargo_mcp::{Config, Server};
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    println!("Starting cargo-mcp with config: {:#?}", config);
    let root = std::env::current_dir()?;
    let service = Server::new(config, root).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
