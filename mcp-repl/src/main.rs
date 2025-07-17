mod client;
mod completer;
mod prompt;
mod repl;
mod term;

use client::{McpClient, Transport};
use crossterm::tty::IsTty;
use repl::Repl;
use std::collections::VecDeque;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("MCP REPL - Model Context Protocol REPL Client");
    println!("=============================================");

    let mut args = std::env::args().skip(1).collect::<VecDeque<_>>();
    let transport = if let Some(first) = args.pop_front() {
        if first.starts_with("http") {
            Transport::Http { url: first }
        } else {
            Transport::Stdio {
                command: first,
                args: args.into(),
            }
        }
    } else {
        return Err(anyhow::anyhow!("No MCP server specified"));
    };

    let client = McpClient::new(transport).await?;

    let server_info = client.server_info();
    println!();
    println!(
        "Connected to: {} v{}",
        server_info.server_info.name, server_info.server_info.version
    );
    println!("Protocol: {}", server_info.protocol_version);
    println!("Available tools: {}", client.tool_names().len());
    println!();
    println!("Type 'help' ('h') for available commands, 'quit' ('q') to exit.");
    println!();

    // Create and run the REPL
    let mut repl = Repl::new(client);

    if std::io::stdin().is_tty() {
        repl.run().await?;
    } else {
        repl.run_non_interactive().await?;
    }

    println!("Goodbye!");
    Ok(())
}
