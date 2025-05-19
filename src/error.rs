//= docs/design/technical-spec.md#error-handling
//# The server MUST return appropriate error responses for all failure cases.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    //= docs/design/technical-spec.md#error-handling
    //# The server MUST return a "not found" error when a requested crate does not exist.
    #[error("Crate not found: {0}")]
    CrateNotFound(String),

    //= docs/design/technical-spec.md#error-handling
    //# The server MUST return an "invalid parameters" error when an invalid version is specified.
    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    //= docs/design/technical-spec.md#error-handling
    //# The server MUST return an "invalid parameters" error when an invalid version is specified.
    #[error("Invalid version format: {0}")]
    InvalidVersionFormat(String),

    //= docs/design/technical-spec.md#security-considerations
    //# The server MUST validate all input parameters to prevent command injection.
    #[error("Invalid input parameter: {0}")]
    InvalidInput(String),

    //= docs/design/technical-spec.md#security-considerations
    //# The server MUST handle file paths securely to prevent path traversal attacks.
    #[error("Invalid file path: {0}")]
    InvalidPath(String),

    //= docs/design/technical-spec.md#error-handling
    //# The server MUST return an "internal error" for command execution failures.
    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    //= docs/design/technical-spec.md#error-handling
    //# The server MUST return an "internal error" for parsing failures.
    #[error("Failed to parse response: {0}")]
    ParseError(String),

    //= docs/design/technical-spec.md#error-handling
    //# The server MUST provide error messages that are helpful for debugging.
    #[error("Documentation generation failed: {0}")]
    DocGenFailed(String),

    //= docs/design/technical-spec.md#error-handling
    //# The server MUST return an "internal error" for command execution failures.
    #[error("Failed to access file system: {0}")]
    IoError(#[from] std::io::Error),

    //= docs/design/technical-spec.md#error-handling
    //# The server MUST return an "internal error" for parsing failures.
    #[error("Failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl From<Error> for rmcp::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::CrateNotFound(name) => rmcp::Error::new(
                rmcp::model::ErrorCode::RESOURCE_NOT_FOUND,
                format!("Crate not found: {}", name),
                None,
            ),
            Error::InvalidVersion(ver) => rmcp::Error::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                format!("Invalid version: {}", ver),
                None,
            ),
            Error::InvalidVersionFormat(msg) => rmcp::Error::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                format!("Invalid version format: {}", msg),
                None,
            ),
            Error::InvalidInput(msg) => rmcp::Error::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                format!("Invalid input: {}", msg),
                None,
            ),
            Error::InvalidPath(path) => rmcp::Error::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                format!("Invalid path: {}", path),
                None,
            ),
            _ => rmcp::Error::new(
                rmcp::model::ErrorCode::INTERNAL_ERROR,
                err.to_string(),
                None,
            ),
        }
    }
}

impl From<public_api::Error> for Error {
    fn from(err: public_api::Error) -> Self {
        Error::DocGenFailed(err.to_string())
    }
}

impl From<rustdoc_json::BuildError> for Error {
    fn from(err: rustdoc_json::BuildError) -> Self {
        Error::DocGenFailed(err.to_string())
    }
}

impl From<cargo_metadata::Error> for Error {
    fn from(err: cargo_metadata::Error) -> Self {
        Error::CommandFailed(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
