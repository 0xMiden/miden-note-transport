use thiserror::Error;

use crate::database::DatabaseError;

/// Main error type
#[derive(Error, Debug)]
pub enum Error {
    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error ([`serde_json::Error`])
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// gRPC error-status
    #[error("gRPC error: {0}")]
    GrpcStatus(Box<tonic::Status>),

    /// gRPC connection error
    #[error("gRPC error: {0}")]
    GrpcTransport(#[from] tonic::transport::Error),

    /// Internal node error
    #[error("Internal server error: {0}")]
    Internal(String),

    /// Generic node error
    #[error("Error: {0}")]
    Generic(#[from] anyhow::Error),
}

impl From<tonic::Status> for Error {
    fn from(status: tonic::Status) -> Self {
        Error::GrpcStatus(Box::new(status))
    }
}

/// Main result type
pub type Result<T> = std::result::Result<T, Error>;
