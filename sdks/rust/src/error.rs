//! Error types for the Onyx SDK.

use thiserror::Error;

/// Errors that can occur when using the Onyx SDK.
#[derive(Error, Debug)]
pub enum OnyxError {
    /// The server returned an HTTP error response.
    #[error("API error ({status}): {message}")]
    ApiError {
        /// HTTP status code.
        status: u16,
        /// Error message from the server.
        message: String,
    },

    /// A network or transport error occurred.
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    /// Failed to serialize or deserialize JSON.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Invalid configuration (e.g. malformed URL).
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// The requested resource was not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// An invalid argument was provided.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// URL parsing error.
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
}

/// Convenience type alias for SDK results.
pub type OnyxResult<T> = Result<T, OnyxError>;
