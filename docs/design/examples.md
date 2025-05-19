# Usage Examples: Rust Documentation MCP Server

This document provides examples of how to interact with the Rust Documentation MCP server from different contexts.

## From an AI Assistant

### 1. Getting Documentation for a Crate

```typescript
// Access documentation for the latest version
const serdeDoc = await mcp.access_resource("crate/serde/docs");
console.log("Serde's public API:", serdeDoc.public_api);

// Access documentation for a specific version
const tokioDoc = await mcp.access_resource("crate/tokio/docs?version=1.0.0");
console.log("Tokio 1.0.0 types:", tokioDoc.types);
```

### 2. Getting Crate Information

```typescript
// Get version and feature information
const axumInfo = await mcp.access_resource("crate/axum/info");
console.log("Latest version:", axumInfo.latest_version);
console.log("Available features:", axumInfo.features);

// Use this information when suggesting dependencies
console.log(`Add this to Cargo.toml:
[dependencies]
axum = { version = "${axumInfo.latest_version}", features = ["macros"] }
`);
```

### 3. Getting Project Metadata

```typescript
// Get current project's metadata
const metadata = await mcp.access_resource("project/metadata");

// Analyze dependencies
console.log("Project dependencies:", metadata.dependencies);

// Check available features
console.log("Available features:", metadata.features);
```

## From Rust Code

### 1. Using as a Client

```rust
use rmcp::{ServiceExt, transport::stdio};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the documentation server
    let service = ()
        .serve(TokioChildProcess::new(Command::new("cargo-mcp"))?)
        .await?;

    // Get documentation for a crate
    let docs: Documentation = service
        .access_resource("crate/serde/docs")
        .await?;
    
    println!("Public API items: {}", docs.public_api.len());

    // Get crate information
    let info: CrateInfo = service
        .access_resource("crate/tokio/info")
        .await?;
    
    println!("Latest version: {}", info.latest_version);

    // Get project metadata
    let metadata: ProjectMetadata = service
        .access_resource("project/metadata")
        .await?;
    
    println!("Workspace members: {:?}", metadata.workspace_members);

    Ok(())
}
```

### 2. Using with Error Handling

```rust
use rmcp::{ServiceExt, Error as McpError};

async fn get_crate_docs(service: &impl Service, name: &str) -> Result<Documentation, McpError> {
    // Try to get documentation
    match service.access_resource(&format!("crate/{}/docs", name)).await {
        Ok(docs) => Ok(docs),
        Err(McpError::NotFound(_)) => {
            eprintln!("Crate '{}' not found", name);
            Err(McpError::not_found(name))
        }
        Err(e) => {
            eprintln!("Error getting docs for '{}': {}", name, e);
            Err(e)
        }
    }
}

async fn get_crate_info(service: &impl Service, name: &str) -> Result<CrateInfo, McpError> {
    // Try to get crate information
    match service.access_resource(&format!("crate/{}/info", name)).await {
        Ok(info) => Ok(info),
        Err(McpError::NotFound(_)) => {
            eprintln!("Crate '{}' not found", name);
            Err(McpError::not_found(name))
        }
        Err(e) => {
            eprintln!("Error getting info for '{}': {}", name, e);
            Err(e)
        }
    }
}
```

## From Command Line

### 1. Using curl with HTTP Transport

```bash
# Get documentation
curl "http://localhost:3000/crate/serde/docs"

# Get specific version
curl "http://localhost:3000/crate/tokio/docs?version=1.0.0"

# Get crate info
curl "http://localhost:3000/crate/axum/info"

# Get project metadata
curl "http://localhost:3000/project/metadata"
```

### 2. Using Direct STDIO Communication

```bash
# Start the server
cargo run | jq

# Send a request (using JSON-RPC format)
echo '{"jsonrpc":"2.0","method":"access_resource","params":{"uri":"crate/serde/docs"},"id":1}'

# Get crate info
echo '{"jsonrpc":"2.0","method":"access_resource","params":{"uri":"crate/tokio/info"},"id":2}'
```

## Common Use Cases

### 1. Adding Dependencies with Features

```typescript
// AI Assistant workflow for adding dependencies
const info = await mcp.access_resource("crate/tokio/info");

// Check if specific features are available
const hasFullFeature = info.features.hasOwnProperty("full");
const hasMacrosFeature = info.features.hasOwnProperty("macros");

// Generate appropriate dependency line
const features = [];
if (hasFullFeature) features.push("full");
if (hasMacrosFeature) features.push("macros");

const dependencyLine = features.length > 0
    ? `tokio = { version = "${info.latest_version}", features = [${features.map(f => `"${f}"`).join(", ")}] }`
    : `tokio = "${info.latest_version}"`;

console.log(`Add this to Cargo.toml:\n[dependencies]\n${dependencyLine}`);
```

### 2. Analyzing API Changes

```typescript
// Get documentation for two versions
const oldDocs = await mcp.access_resource("crate/serde/docs?version=1.0.0");
const newDocs = await mcp.access_resource("crate/serde/docs?version=1.0.1");

// Compare public APIs
const addedItems = newDocs.public_api.filter(item => 
    !oldDocs.public_api.some(old => old.name === item.name)
);

const removedItems = oldDocs.public_api.filter(item =>
    !newDocs.public_api.some(new => new.name === item.name)
);

console.log("Added items:", addedItems);
console.log("Removed items:", removedItems);
```

### 3. Workspace Analysis

```typescript
// Get project metadata
const metadata = await mcp.access_resource("project/metadata");

// Analyze workspace structure
const workspaceMembers = metadata.workspace_members;
const sharedDependencies = new Set();

// Find dependencies shared across workspace
workspaceMembers.forEach(member => {
    member.dependencies.forEach(dep => {
        if (workspaceMembers.every(m => 
            m.dependencies.some(d => d.name === dep.name)
        )) {
            sharedDependencies.add(dep.name);
        }
    });
});

console.log("Dependencies shared across workspace:", sharedDependencies);
