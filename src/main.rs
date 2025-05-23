use cargo_mcp::Config;
use cargo_mcp::Server;
use cargo_mcp::providers::Providers;
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    eprintln!("Starting cargo-mcp with config: {:#?}", config);

    // Install nightly toolchain
    rustup_toolchain::install(public_api::MINIMUM_NIGHTLY_RUST_VERSION)?;

    let providers = Providers::new(&config);
    let service = Server::new(providers).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
