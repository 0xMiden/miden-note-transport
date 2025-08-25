use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("gRPC error: {0}")]
    GrpcStatus(Box<tonic::Status>),

    #[error("gRPC error: {0}")]
    GrpcTransport(#[from] tonic::transport::Error),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Error: {0}")]
    Generic(#[from] anyhow::Error),
}

impl From<tonic::Status> for Error {
    fn from(status: tonic::Status) -> Self {
        Error::GrpcStatus(Box::new(status))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
