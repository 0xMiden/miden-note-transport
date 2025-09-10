use alloc::{boxed::Box, string::String};

use thiserror::Error;

use crate::database::DatabaseError;

/// Main error type
#[derive(Error, Debug)]
pub enum Error {
    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    /// IO error
    #[cfg(feature = "std")]
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error ([`serde_json::Error`])
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// gRPC error-status
    #[error("gRPC error: {0}")]
    GrpcStatus(Box<tonic::Status>),

    /// gRPC transport error
    #[cfg(feature = "tonic")]
    #[error("gRPC error: {0}")]
    GrpcTransport(#[from] tonic::transport::Error),

    /// Invalid note error
    #[error("Invalid note data: {0}")]
    InvalidNoteData(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Invalid tag error
    #[error("Invalid tag: {0}")]
    InvalidTag(String),

    /// Internal client error
    #[error("Internal server error: {0}")]
    Internal(String),

    /// Generic error
    #[error("Error: {0}")]
    Generic(#[from] anyhow::Error),
}

impl From<tonic::Status> for Error {
    fn from(status: tonic::Status) -> Self {
        Error::GrpcStatus(Box::new(status))
    }
}

/// Main result type
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(feature = "idxdb")]
impl From<Error> for wasm_bindgen::JsValue {
    fn from(e: Error) -> Self {
        use alloc::string::ToString;
        Self::from_str(&e.to_string())
    }
}
