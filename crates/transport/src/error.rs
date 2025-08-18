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

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("Note not found: {0}")]
    NoteNotFound(String),

    #[error("Invalid note data: {0}")]
    InvalidNoteData(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Invalid tag: {0}")]
    InvalidTag(String),

    #[error("Note too large: max size is {max_size} bytes, got {actual_size} bytes")]
    NoteTooLarge { max_size: usize, actual_size: usize },

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
