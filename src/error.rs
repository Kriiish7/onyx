use thiserror::Error;

/// Central error type for Onyx operations.
#[derive(Error, Debug)]
pub enum OnyxError {
    #[error("Node not found: {0}")]
    NodeNotFound(uuid::Uuid),

    #[error("Edge not found: {0}")]
    EdgeNotFound(uuid::Uuid),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("Branch already exists: {0}")]
    BranchAlreadyExists(String),

    #[error("Duplicate node ID: {0}")]
    DuplicateNode(uuid::Uuid),

    #[error("Duplicate edge ID: {0}")]
    DuplicateEdge(uuid::Uuid),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Embedding dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Ingestion error: {0}")]
    IngestionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

/// Convenience type alias for Onyx results.
pub type OnyxResult<T> = Result<T, OnyxError>;
