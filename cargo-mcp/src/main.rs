use cargo_mcp::{Config, Server, providers::Providers};
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Change directories to the home to avoid rustup/cargo overrides or project interactions
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let _ = std::env::set_current_dir(home);

    let config = Config::default();
    eprintln!("Starting cargo-mcp with config: {config:#?}");

    let providers = Providers::new(&config);
    let service = Server::new(providers).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
