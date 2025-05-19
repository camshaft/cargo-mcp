# Implementation Guide: Rust Documentation MCP Server

This guide provides implementation details and code examples for the requirements specified in the technical specification.

## Server Implementation

### Core Types

```rust
/// Main server type holding caches and state
pub struct RustDocServer {
    doc_cache: Arc<Mutex<HashMap<String, CacheEntry<Documentation>>>>,
    info_cache: Arc<Mutex<HashMap<String, CacheEntry<CrateInfo>>>>,
}

/// Cache entry with TTL
struct CacheEntry<T> {
    data: T,
    timestamp: SystemTime,
    ttl: Duration,
}

/// Documentation response structure
pub struct Documentation {
    pub public_api: Vec<ApiItem>,
    pub modules: Vec<Module>,
    pub types: Vec<Type>,
    pub traits: Vec<Trait>,
}

/// API item representing a single documented item
pub struct ApiItem {
    pub name: String,
    pub kind: ApiItemKind,
    pub visibility: Visibility,
    pub documentation: Option<String>,
    pub attributes: Vec<Attribute>,
}

/// Crate information response
pub struct CrateInfo {
    pub latest_version: String,
    pub all_versions: Vec<String>,
    pub features: HashMap<String, Vec<String>>,
    pub dependencies: Vec<Dependency>,
    pub description: Option<String>,
    pub repository: Option<String>,
}

/// Project metadata response
pub struct ProjectMetadata {
    pub workspace_members: Vec<Package>,
    pub dependencies: Vec<Dependency>,
    pub targets: Vec<Target>,
    pub features: HashMap<String, Vec<String>>,
}
```

### Resource Implementations

#### 1. Crate Documentation Resource

```rust
impl RustDocServer {
    //= docs/design/technical-spec.md#crate-documentation-resource
    //# The server MUST provide a resource at path `crate/{name}/docs`.
    //# The server MUST accept an optional version parameter in the query string.
    async fn handle_docs_resource(&self, name: &str, version: Option<&str>) -> Result<Documentation, McpError> {
        // Check cache first
        if let Some(cached) = self.get_from_cache(name, version).await {
            return Ok(cached);
        }

        //= docs/design/technical-spec.md#crate-documentation-resource
        //# The server MUST generate rustdoc JSON using the nightly toolchain.
        //# The server MUST parse the rustdoc JSON using the public-api crate.
        let rustdoc_json = rustdoc_json::Builder::default()
            .toolchain(public_api::MINIMUM_NIGHTLY_RUST_VERSION)
            .build()?;

        let public_api = public_api::Builder::from_rustdoc_json(rustdoc_json)
            .build()?;

        let doc = Documentation::from(public_api);

        //= docs/design/technical-spec.md#crate-documentation-resource
        //# The server SHOULD cache documentation results to improve performance.
        self.cache_documentation(name, version, doc.clone()).await;

        Ok(doc)
    }
}
```

#### 2. Crate Info Resource

```rust
impl RustDocServer {
    //= docs/design/technical-spec.md#crate-information-resource
    //# The server MUST provide a resource at path `crate/{name}/info`.
    //# The server MUST execute the `cargo info` command to retrieve crate information.
    async fn handle_info_resource(&self, name: &str) -> Result<CrateInfo, McpError> {
        if let Some(cached) = self.get_info_from_cache(name).await {
            return Ok(cached);
        }

        let output = Command::new("cargo")
            .arg("info")
            .arg("--color=never")
            .arg(name)
            .output()
            .await?;

        let info = self.parse_cargo_info_output(&output.stdout)?;

        //= docs/design/technical-spec.md#crate-information-resource
        //# The server SHOULD cache crate information with a shorter TTL than documentation.
        self.cache_info(name, info.clone()).await;

        Ok(info)
    }
}
```

#### 3. Project Metadata Resource

```rust
impl RustDocServer {
    //= docs/design/technical-spec.md#project-metadata-resource
    //# The server MUST provide a resource at path `project/metadata`.
    //# The server MUST execute the `cargo metadata` command with format version 1.
    async fn handle_metadata_resource(&self) -> Result<ProjectMetadata, McpError> {
        let output = Command::new("cargo")
            .arg("metadata")
            .arg("--format-version=1")
            .output()
            .await?;

        let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)?;
        Ok(ProjectMetadata::from(metadata))
    }
}
```

### Cache Implementation

```rust
impl RustDocServer {
    //= docs/design/technical-spec.md#caching-requirements
    //# The server MUST implement a caching system for documentation and crate information.
    //# The server MUST store cache entries with timestamps.
    async fn get_from_cache(&self, name: &str, version: Option<&str>) -> Option<Documentation> {
        let cache = self.doc_cache.lock().await;
        let key = Self::cache_key(name, version);
        
        if let Some(entry) = cache.get(&key) {
            //= docs/design/technical-spec.md#caching-requirements
            //# The server MUST validate cache entries before returning them.
            if entry.is_valid() {
                return Some(entry.data.clone());
            }
        }
        None
    }

    async fn cache_documentation(&self, name: &str, version: Option<&str>, doc: Documentation) {
        let mut cache = self.doc_cache.lock().await;
        let key = Self::cache_key(name, version);
        
        //= docs/design/technical-spec.md#caching-requirements
        //# The server MUST implement time-to-live (TTL) for cache entries.
        cache.insert(key, CacheEntry::new(doc, Duration::from_secs(3600)));
    }
}

impl<T> CacheEntry<T> {
    fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            timestamp: SystemTime::now(),
            ttl,
        }
    }

    fn is_valid(&self) -> bool {
        SystemTime::now()
            .duration_since(self.timestamp)
            .map(|age| age < self.ttl)
            .unwrap_or(false)
    }
}
```

### Error Handling

```rust
//= docs/design/technical-spec.md#error-handling
//# The server MUST return appropriate error responses for all failure cases.
#[derive(Debug, Error)]
pub enum RustDocError {
    //# The server MUST return a "not found" error when a requested crate does not exist.
    #[error("Crate not found: {0}")]
    CrateNotFound(String),

    //# The server MUST return an "invalid parameters" error when an invalid version is specified.
    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    //# The server MUST return an "internal error" for command execution failures.
    #[error("Command failed: {0}")]
    CommandFailed(#[from] std::io::Error),

    //# The server MUST return an "internal error" for parsing failures.
    #[error("Parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Documentation generation failed: {0}")]
    DocGenFailed(String),
}

impl From<RustDocError> for McpError {
    fn from(err: RustDocError) -> Self {
        match err {
            RustDocError::CrateNotFound(name) => McpError::not_found(name),
            RustDocError::InvalidVersion(ver) => McpError::invalid_params(ver),
            _ => McpError::internal_error(err.to_string()),
        }
    }
}
```

### Configuration

```rust
//= docs/design/technical-spec.md#configuration
//# The server MUST support configuration of cache TTL values.
//# The server MUST support configuration of maximum cache size.
#[derive(Debug, Clone)]
pub struct Config {
    pub doc_cache_ttl: Duration,
    pub info_cache_ttl: Duration,
    pub max_cache_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            doc_cache_ttl: Duration::from_secs(3600),
            info_cache_ttl: Duration::from_secs(300),
            max_cache_size: 100,
        }
    }
}
