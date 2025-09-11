use alloc::string::{String, ToString};

/// Errors generated from the `SQLite` store.
#[derive(Debug, thiserror::Error)]
pub enum SqliteStoreError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Migration error: {0}")]
    MigrationError(String),
    #[error("Schema version mismatch")]
    SchemaVersionMismatch,
    #[error("No settings table in the database")]
    MissingSettingsTable,
    #[error("Migration hashes mismatch")]
    MigrationHashMismatch,
    #[error("Failed to decode hex string: {0}")]
    HexDecodeError(String),
}

impl From<rusqlite::Error> for SqliteStoreError {
    fn from(err: rusqlite::Error) -> Self {
        SqliteStoreError::DatabaseError(err.to_string())
    }
}

impl From<rusqlite_migration::Error> for SqliteStoreError {
    fn from(err: rusqlite_migration::Error) -> Self {
        SqliteStoreError::MigrationError(err.to_string())
    }
}

impl From<SqliteStoreError> for super::super::DatabaseError {
    fn from(err: SqliteStoreError) -> Self {
        match err {
            SqliteStoreError::DatabaseError(msg) => Self::Protocol(msg),
            SqliteStoreError::MigrationError(msg) => Self::Configuration(msg),
            SqliteStoreError::HexDecodeError(msg) => Self::Encoding(msg),
            SqliteStoreError::SchemaVersionMismatch => {
                Self::Configuration("Schema version mismatch".to_string())
            },
            SqliteStoreError::MigrationHashMismatch => {
                Self::Configuration("Migration hash mismatch".to_string())
            },
            SqliteStoreError::MissingSettingsTable => {
                Self::Configuration("No settings table in the database".to_string())
            },
        }
    }
}
