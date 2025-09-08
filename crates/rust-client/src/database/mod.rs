#[cfg(feature = "idxdb")]
pub mod idxdb;
#[cfg(feature = "sqlite")]
pub mod sqlite;

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

use chrono::{DateTime, Utc};
use miden_objects::note::{NoteHeader, NoteId, NoteTag};

/// Trait for client database operations
#[cfg_attr(not(feature = "idxdb"), async_trait::async_trait)]
#[cfg_attr(feature = "idxdb", async_trait::async_trait(?Send))]
pub trait DatabaseBackend: Send + Sync {
    /// Store a note
    async fn store_note(
        &self,
        header: &NoteHeader,
        details: &[u8],
        created_at: DateTime<Utc>,
    ) -> Result<(), DatabaseError>;

    /// Get a stored note by ID
    async fn get_stored_note(&self, note_id: &NoteId) -> Result<Option<StoredNote>, DatabaseError>;

    /// Get all stored notes with provided tag
    async fn get_stored_notes_for_tag(
        &self,
        tag: NoteTag,
    ) -> Result<Vec<StoredNote>, DatabaseError>;

    /// Record that a note has been fetched
    async fn record_fetched_note(
        &self,
        note_id: &NoteId,
        tag: NoteTag,
    ) -> Result<(), DatabaseError>;

    /// Check if a note has been fetched before
    async fn note_fetched(&self, note_id: &NoteId) -> Result<bool, DatabaseError>;

    /// Get all fetched note IDs for a specific tag
    async fn get_fetched_notes_for_tag(&self, tag: NoteTag) -> Result<Vec<NoteId>, DatabaseError>;

    /// Get database statistics
    async fn get_stats(&self) -> Result<DatabaseStats, DatabaseError>;

    /// Clean up old data based on retention policy
    async fn cleanup_old_data(&self, retention_days: u32) -> Result<u64, DatabaseError>;
}

/// Client database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_note_size: usize,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite::memory:".to_string(),
            max_note_size: 1024 * 1024, // 1MB default
        }
    }
}

/// Client database for storing persistent state
pub struct Database {
    backend: Box<dyn DatabaseBackend>,
}

impl Database {
    /// Create a new client database with the specified backend
    pub fn new(backend: Box<dyn DatabaseBackend>) -> Self {
        Self { backend }
    }

    #[cfg(feature = "sqlite")]
    /// Create a new SQLite-based client database
    pub async fn new_sqlite(config: DatabaseConfig) -> Result<Self, DatabaseError> {
        let backend = sqlite::SqliteDatabase::connect(config).await?;
        Ok(Self::new(Box::new(backend)))
    }

    /// Store an encrypted note
    pub async fn store_note(
        &self,
        header: &NoteHeader,
        encrypted_data: &[u8],
        created_at: DateTime<Utc>,
    ) -> Result<(), DatabaseError> {
        self.backend.store_note(header, encrypted_data, created_at).await
    }

    /// Get an stored note by ID
    pub async fn get_stored_note(
        &self,
        note_id: &NoteId,
    ) -> Result<Option<StoredNote>, DatabaseError> {
        self.backend.get_stored_note(note_id).await
    }

    /// Get all stored notes for a tag
    pub async fn get_stored_notes_for_tag(
        &self,
        tag: NoteTag,
    ) -> Result<Vec<StoredNote>, DatabaseError> {
        self.backend.get_stored_notes_for_tag(tag).await
    }

    /// Record that a note has been fetched
    pub async fn record_fetched_note(
        &self,
        note_id: &NoteId,
        tag: NoteTag,
    ) -> Result<(), DatabaseError> {
        self.backend.record_fetched_note(note_id, tag).await
    }

    /// Check if a note has been fetched before
    pub async fn note_fetched(&self, note_id: &NoteId) -> Result<bool, DatabaseError> {
        self.backend.note_fetched(note_id).await
    }

    /// Get all fetched note IDs for a specific tag
    pub async fn get_fetched_notes_for_tag(
        &self,
        tag: NoteTag,
    ) -> Result<Vec<NoteId>, DatabaseError> {
        self.backend.get_fetched_notes_for_tag(tag).await
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        self.backend.get_stats().await
    }

    /// Clean up old data based on retention policy
    pub async fn cleanup_old_data(&self, retention_days: u32) -> Result<u64, DatabaseError> {
        self.backend.cleanup_old_data(retention_days).await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Configuration error: {0}")]
    Configuration(String),
    #[error("Encoding error: {0}")]
    Encoding(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("{0}")]
    Generic(#[from] anyhow::Error),
}

/// Encrypted note stored in the client database
#[derive(Debug, Clone)]
pub struct StoredNote {
    pub header: NoteHeader,
    pub details: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

/// Client database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    /// Downloaded notes
    pub fetched_notes_count: u64,
    /// Stored (kept) notes
    pub stored_notes_count: u64,
    /// Stored tags
    pub unique_tags_count: u64,
}
