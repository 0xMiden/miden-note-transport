mod sqlite;

use self::sqlite::SQLiteDB;
use crate::{
    types::{NoteId, NoteTag, StoredNote, UserId},
    Result,
};

/// Database operations
#[async_trait::async_trait]
pub trait DatabaseBackend: Send + Sync {
    /// Connect to the database
    async fn connect(config: DatabaseConfig) -> Result<Self>
    where
        Self: Sized;

    /// Store a new note
    async fn store_note(&self, note: &StoredNote) -> Result<()>;

    /// Fetch notes by tag
    async fn fetch_notes(&self, tag: NoteTag, user_id: Option<UserId>) -> Result<Vec<StoredNote>>;

    /// Mark a note as received by a user
    async fn mark_received(&self, note_id: NoteId, user_id: UserId) -> Result<()>;

    /// Get statistics about the database
    async fn get_stats(&self) -> Result<(u64, u64)>;

    /// Clean up old notes based on retention policy
    async fn cleanup_old_notes(&self, retention_days: u32) -> Result<u64>;

    /// Check if a note exists
    async fn note_exists(&self, note_id: NoteId) -> Result<bool>;
}

/// Database manager for the transport layer
pub struct Database {
    backend: Box<dyn DatabaseBackend>,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_note_size: usize,
    pub retention_days: u32,
    pub rate_limit_per_minute: u32,
    pub request_timeout_seconds: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite::memory:".to_string(),
            retention_days: 30,
            rate_limit_per_minute: 100,
            request_timeout_seconds: 10,
            max_note_size: 1024 * 1024,
        }
    }
}

impl Database {
    /// Connect to a database with SQLite backend
    pub async fn connect(config: DatabaseConfig) -> Result<Self> {
        let backend = SQLiteDB::connect(config).await?;
        Ok(Self {
            backend: Box::new(backend),
        })
    }

    /// Store a new note
    pub async fn store_note(&self, note: &StoredNote) -> Result<()> {
        self.backend.store_note(note).await
    }

    /// Fetch notes by tag, optionally filtered by block number
    pub async fn fetch_notes(
        &self,
        tag: NoteTag,
        user_id: Option<UserId>,
    ) -> Result<Vec<StoredNote>> {
        self.backend.fetch_notes(tag, user_id).await
    }

    /// Mark a note as received by a user
    pub async fn mark_received(
        &self,
        note_id: miden_objects::note::NoteId,
        user_id: UserId,
    ) -> Result<()> {
        self.backend.mark_received(note_id, user_id).await
    }

    /// Get statistics about the database
    pub async fn get_stats(&self) -> Result<(u64, u64)> {
        self.backend.get_stats().await
    }

    /// Clean up old notes based on retention policy
    pub async fn cleanup_old_notes(&self, retention_days: u32) -> Result<u64> {
        self.backend.cleanup_old_notes(retention_days).await
    }

    /// Check if a note exists
    pub async fn note_exists(&self, note_id: miden_objects::note::NoteId) -> Result<bool> {
        self.backend.note_exists(note_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{test_note_header, EncryptedDetails, TEST_TAG};
    use chrono::Utc;

    #[tokio::test]
    async fn test_sqlite_database() {
        let db = Database::connect(DatabaseConfig::default()).await.unwrap();
        let user1 = UserId::random();

        let note = StoredNote {
            header: test_note_header(),
            encrypted_data: EncryptedDetails(vec![1, 2, 3, 4]),
            created_at: Utc::now(),
            received_by: None,
        };

        db.store_note(&note).await.unwrap();

        let fetched_notes = db.fetch_notes(TEST_TAG.into(), user1.into()).await.unwrap();
        assert_eq!(fetched_notes.len(), 1);
        assert_eq!(fetched_notes[0].header.id(), note.header.id());

        // Test note exists
        assert!(db.note_exists(note.header.id()).await.unwrap());

        // Test stats
        let (total_notes, total_tags) = db.get_stats().await.unwrap();
        assert_eq!(total_notes, 1);
        assert_eq!(total_tags, 1);
    }

    #[tokio::test]
    async fn test_mark_received() {
        let db = Database::connect(DatabaseConfig::default()).await.unwrap();
        let user1 = UserId::random();
        let user2 = UserId::random();

        let note = StoredNote {
            header: test_note_header(),
            encrypted_data: EncryptedDetails(vec![9, 10, 11, 12]),
            created_at: Utc::now(),
            received_by: None,
        };

        db.store_note(&note).await.unwrap();

        let fetched_notes = db
            .fetch_notes(TEST_TAG.into(), user1.clone().into())
            .await
            .unwrap();
        assert_eq!(fetched_notes.len(), 1);

        // Mark as received
        db.mark_received(note.header.id(), user1.clone())
            .await
            .unwrap();
        db.mark_received(note.header.id(), user2.clone())
            .await
            .unwrap();

        // Fetch and verify received_by
        let fetched_notes = db
            .fetch_notes(TEST_TAG.into(), user1.clone().into())
            .await
            .unwrap();
        assert_eq!(fetched_notes.len(), 0);
        let fetched_notes_user2 = db
            .fetch_notes(TEST_TAG.into(), user2.clone().into())
            .await
            .unwrap();
        assert_eq!(fetched_notes_user2.len(), 0);

        // Fetch without user_id filter
        let fetched_notes_all = db.fetch_notes(TEST_TAG.into(), None).await.unwrap();
        assert_eq!(fetched_notes_all.len(), 1);
    }
}
