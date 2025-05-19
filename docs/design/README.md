# Rust Documentation MCP Server Design

This document outlines the design of the Rust Documentation MCP server, which provides access to crate documentation, version information, and project metadata through the Model Context Protocol.

## Architecture

```mermaid
graph TD
    A[RustDocServer] --> B[Resources]
    
    B --> C[crate/{name}/docs]
    B --> D[crate/{name}/info]
    B --> E[project/metadata]
    
    C --> F[public-api Analysis]
    F --> G[rustdoc JSON]
    G --> H[rustdoc +nightly]
    
    D --> I[cargo info]
    E --> J[cargo metadata]
```

## Resources

The server exposes three main resources:

### 1. Crate Documentation (`crate/{name}/docs`)

Provides structured documentation for a specified crate using the public-api crate to analyze rustdoc JSON output.

**URI Format:** `crate/{name}/docs?version={version}`

**Response Structure:**
```rust
struct Documentation {
    public_api: Vec<ApiItem>,
    modules: Vec<Module>,
    types: Vec<Type>,
    traits: Vec<Trait>,
}
```

### 2. Crate Info (`crate/{name}/info`)

Provides version information and available features for a specified crate using `cargo info`.

**URI Format:** `crate/{name}/info`

**Response Structure:**
```rust
struct CrateInfo {
    latest_version: String,
    all_versions: Vec<String>,
    features: HashMap<String, Vec<String>>,
    dependencies: Vec<Dependency>,
    description: Option<String>,
    repository: Option<String>,
}
```

### 3. Project Metadata (`project/metadata`)

Provides metadata about the current project using `cargo metadata`.

**URI Format:** `project/metadata`

**Response Structure:**
```rust
struct ProjectMetadata {
    workspace_members: Vec<Package>,
    dependencies: Vec<Dependency>,
    targets: Vec<Target>,
    features: HashMap<String, Vec<String>>,
}
```

## Implementation Details

### Server Implementation

```rust
#[derive(Clone)]
pub struct RustDocServer {
    doc_cache: Arc<Mutex<HashMap<String, Documentation>>>,
    info_cache: Arc<Mutex<HashMap<String, CrateInfo>>>,
}

#[tool(tool_box)]
impl rmcp::ServerHandler for RustDocServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Rust crate documentation and metadata server".into()),
            capabilities: ServerCapabilities::builder()
                .enable_resources()
                .build(),
            ..Default::default()
        }
    }
}
```

### Resource Handlers

Each resource is implemented as an async method:

```rust
impl RustDocServer {
    async fn handle_docs_resource(&self, name: &str, version: Option<&str>) -> Result<Documentation, McpError> {
        // Documentation resource implementation
    }

    async fn handle_info_resource(&self, name: &str) -> Result<CrateInfo, McpError> {
        // Info resource implementation
    }

    async fn handle_metadata_resource(&self) -> Result<ProjectMetadata, McpError> {
        // Metadata resource implementation
    }
}
```

### Caching Strategy

- Documentation is cached per crate/version combination
- Crate info is cached with a TTL to ensure freshness
- Project metadata is not cached as it's generated on demand

### Dependencies

```toml
[dependencies]
rmcp = { version = "0.1.5", features = ["server"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
public-api = "0.3"
rustup-toolchain = "0.1"
rustdoc-json = "0.8"
```

## Error Handling

The server handles various error cases:
- Missing crates
- Invalid versions
- Command execution failures
- Parse errors
- Cache failures

All errors are mapped to appropriate MCP error types for consistent client handling.

## Implementation Status

### Completed Features

1. Core Infrastructure ✓
   - Basic MCP server implementation
   - Resource URI routing
   - Server handler trait implementation
   - Type definitions and data structures

2. Caching System ✓
   - TTL-based caching implementation
   - LRU eviction policy
   - Thread-safe concurrent access
   - Separate caches for docs and info
   - Cache size limits

3. Security Features ✓
   - Input validation for crate names
   - Safe file path handling
   - Basic error handling

### In Progress Features

1. Documentation Generation 🚧
   - Basic rustdoc JSON generation structure
   - Need to implement version-specific documentation
   - Need to properly use public_api::MINIMUM_NIGHTLY_RUST_VERSION

2. Testing Infrastructure 🚧
   - Basic test structure in place
   - Need comprehensive test coverage
   - Need integration tests
   - Need performance tests

### Planned Features

1. Error Handling Improvements
   - More specific error types
   - Better error messages and context
   - Logging system integration
   - Error recovery strategies

2. Performance Optimizations
   - Rate limiting implementation
   - Background cache warming
   - Concurrent request optimization
   - Resource usage monitoring

3. Documentation
   - API documentation improvements
   - Usage examples
   - Integration guides
   - Performance recommendations

## Usage Examples

### Getting Documentation
```rust
// Access crate documentation
let docs = mcp.access_resource("crate/serde/docs").await?;

// Access specific version
let docs = mcp.access_resource("crate/serde/docs?version=1.0.0").await?;
```

### Getting Crate Info
```rust
// Get crate information including versions and features
let info = mcp.access_resource("crate/tokio/info").await?;
```

### Getting Project Metadata
```rust
// Get current project metadata
let metadata = mcp.access_resource("project/metadata").await?;
