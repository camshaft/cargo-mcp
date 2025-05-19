use std::time::Duration;

//= docs/design/technical-spec.md#configuration
//# The server MUST support configuration of cache TTL values.

//= docs/design/technical-spec.md#configuration
//# The server MUST support configuration of maximum cache size.

#[derive(Debug, Clone)]
pub struct Config {
    /// TTL for documentation cache entries
    pub doc_cache_ttl: Duration,

    /// TTL for crate info cache entries
    pub info_cache_ttl: Duration,

    /// Maximum number of entries in each cache
    pub max_cache_size: usize,

    /// Path to nightly toolchain for rustdoc
    pub nightly_toolchain: Option<String>,

    /// Path to cargo executable
    pub cargo_path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Default TTL for documentation: 1 hour
            doc_cache_ttl: Duration::from_secs(3600),

            // Default TTL for crate info: 5 minutes
            info_cache_ttl: Duration::from_secs(300),

            // Default maximum cache size: 100 entries
            max_cache_size: 100,

            // Default to None - will use public-api's default
            nightly_toolchain: None,

            // Default to None - will use system cargo
            cargo_path: None,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_doc_cache_ttl(mut self, ttl: Duration) -> Self {
        self.doc_cache_ttl = ttl;
        self
    }

    pub fn with_info_cache_ttl(mut self, ttl: Duration) -> Self {
        self.info_cache_ttl = ttl;
        self
    }

    pub fn with_max_cache_size(mut self, size: usize) -> Self {
        self.max_cache_size = size;
        self
    }

    pub fn with_nightly_toolchain(mut self, toolchain: impl Into<String>) -> Self {
        self.nightly_toolchain = Some(toolchain.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.doc_cache_ttl, Duration::from_secs(3600));
        assert_eq!(config.info_cache_ttl, Duration::from_secs(300));
        assert_eq!(config.max_cache_size, 100);
        assert!(config.nightly_toolchain.is_none());
    }

    #[test]
    fn test_config_builder() {
        let config = Config::new()
            .with_doc_cache_ttl(Duration::from_secs(7200))
            .with_info_cache_ttl(Duration::from_secs(600))
            .with_max_cache_size(200)
            .with_nightly_toolchain("nightly-2025-05-19");

        assert_eq!(config.doc_cache_ttl, Duration::from_secs(7200));
        assert_eq!(config.info_cache_ttl, Duration::from_secs(600));
        assert_eq!(config.max_cache_size, 200);
        assert_eq!(
            config.nightly_toolchain,
            Some("nightly-2025-05-19".to_string())
        );
    }
}
