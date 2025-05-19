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
    //# The server MUST return an "internal error" for command execution failures.
    #[error("Command failed: {0}")]
    CommandFailed(#[from] std::io::Error),

    //= docs/design/technical-spec.md#error-handling
    //# The server MUST return an "internal error" for parsing failures.
    #[error("Parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    //= docs/design/technical-spec.md#error-handling
    //# The server MUST provide error messages that are helpful for debugging.
    #[error("Documentation generation failed: {0}")]
    DocGenFailed(String),
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

impl From<cargo_metadata::Error> for Error {
    fn from(err: cargo_metadata::Error) -> Self {
        Error::DocGenFailed(err.to_string())
    }
}

impl From<rustdoc_json::BuildError> for Error {
    fn from(err: rustdoc_json::BuildError) -> Self {
        Error::DocGenFailed(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
