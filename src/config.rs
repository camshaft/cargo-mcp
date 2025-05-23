use std::{path::Path, sync::Arc};

#[derive(Debug, Clone)]
pub struct Config {
    /// Path to nightly toolchain for rustdoc
    pub nightly_toolchain: Option<String>,

    /// Path to cargo executable
    pub cargo_path: Option<String>,

    pub pwd: Arc<Path>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Default to None - will use public-api's default
            nightly_toolchain: None,

            // Default to None - will use system cargo
            cargo_path: None,

            pwd: Arc::from(std::env::current_dir().unwrap()),
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
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
        assert!(config.nightly_toolchain.is_none());
    }

    #[test]
    fn test_config_builder() {
        let config = Config::new().with_nightly_toolchain("nightly-2025-05-19");

        assert_eq!(
            config.nightly_toolchain,
            Some("nightly-2025-05-19".to_string())
        );
    }
}
