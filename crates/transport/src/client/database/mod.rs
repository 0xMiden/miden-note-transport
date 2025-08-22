use chrono::{DateTime, Utc};
use miden_objects::{
    account::AccountId,
    note::{NoteHeader, NoteId, NoteTag},
};

use crate::{Result, client::crypto::SerializableKey};

pub mod sqlite;

/// Trait for client database operations
#[async_trait::async_trait]
pub trait ClientDatabaseBackend: Send + Sync {
    /// Store a key for an account ID
    async fn store_key(&self, account_id: &AccountId, key: &SerializableKey) -> Result<()>;

    /// Get a key for an account ID
    async fn get_key(&self, account_id: &AccountId) -> Result<Option<SerializableKey>>;

    /// Get all stored keys
    async fn get_all_keys(&self) -> Result<Vec<(AccountId, SerializableKey)>>;

    /// Store an encrypted note
    async fn store_encrypted_note(
        &self,
        note_id: &NoteId,
        tag: NoteTag,
        header: &NoteHeader,
        encrypted_data: &[u8],
        created_at: DateTime<Utc>,
    ) -> Result<()>;

    /// Get an encrypted note by ID
    async fn get_encrypted_note(&self, note_id: &NoteId) -> Result<Option<EncryptedNote>>;

    /// Get all encrypted notes for a tag
    async fn get_encrypted_notes_for_tag(&self, tag: NoteTag) -> Result<Vec<EncryptedNote>>;

    /// Record that a note has been fetched
    async fn record_fetched_note(&self, note_id: &NoteId, tag: NoteTag) -> Result<()>;

    /// Check if a note has been fetched before
    async fn note_fetched(&self, note_id: &NoteId) -> Result<bool>;

    /// Get all fetched note IDs for a specific tag
    async fn get_fetched_notes_for_tag(&self, tag: NoteTag) -> Result<Vec<NoteId>>;

    /// Get database statistics
    async fn get_stats(&self) -> Result<ClientDatabaseStats>;

    /// Clean up old data based on retention policy
    async fn cleanup_old_data(&self, retention_days: u32) -> Result<u64>;
}

/// Client database configuration
#[derive(Debug, Clone)]
pub struct ClientDatabaseConfig {
    pub database_path: String,
    pub max_note_size: usize,
}

impl Default for ClientDatabaseConfig {
    fn default() -> Self {
        Self {
            database_path: ":memory:".to_string(),
            max_note_size: 1024 * 1024, // 1MB default
        }
    }
}

/// Client database for storing persistent state
pub struct ClientDatabase {
    backend: Box<dyn ClientDatabaseBackend>,
}

impl ClientDatabase {
    /// Create a new client database with the specified backend
    pub fn new(backend: Box<dyn ClientDatabaseBackend>) -> Self {
        Self { backend }
    }

    /// Create a new SQLite-based client database
    pub async fn new_sqlite(config: ClientDatabaseConfig) -> Result<Self> {
        let backend = sqlite::SqliteClientDatabase::connect(config).await?;
        Ok(Self::new(Box::new(backend)))
    }

    /// Store a key for an account ID
    pub async fn store_key(&self, account_id: &AccountId, key: &SerializableKey) -> Result<()> {
        self.backend.store_key(account_id, key).await
    }

    /// Get a key for an account ID
    pub async fn get_key(&self, account_id: &AccountId) -> Result<Option<SerializableKey>> {
        self.backend.get_key(account_id).await
    }

    /// Get all stored keys
    pub async fn get_all_keys(&self) -> Result<Vec<(AccountId, SerializableKey)>> {
        self.backend.get_all_keys().await
    }

    /// Store an encrypted note
    pub async fn store_encrypted_note(
        &self,
        note_id: &NoteId,
        tag: NoteTag,
        header: &NoteHeader,
        encrypted_data: &[u8],
        created_at: DateTime<Utc>,
    ) -> Result<()> {
        self.backend
            .store_encrypted_note(note_id, tag, header, encrypted_data, created_at)
            .await
    }

    /// Get an encrypted note by ID
    pub async fn get_encrypted_note(&self, note_id: &NoteId) -> Result<Option<EncryptedNote>> {
        self.backend.get_encrypted_note(note_id).await
    }

    /// Get all encrypted notes for a tag
    pub async fn get_encrypted_notes_for_tag(&self, tag: NoteTag) -> Result<Vec<EncryptedNote>> {
        self.backend.get_encrypted_notes_for_tag(tag).await
    }

    /// Record that a note has been fetched
    pub async fn record_fetched_note(&self, note_id: &NoteId, tag: NoteTag) -> Result<()> {
        self.backend.record_fetched_note(note_id, tag).await
    }

    /// Check if a note has been fetched before
    pub async fn note_fetched(&self, note_id: &NoteId) -> Result<bool> {
        self.backend.note_fetched(note_id).await
    }

    /// Get all fetched note IDs for a specific tag
    pub async fn get_fetched_notes_for_tag(&self, tag: NoteTag) -> Result<Vec<NoteId>> {
        self.backend.get_fetched_notes_for_tag(tag).await
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<ClientDatabaseStats> {
        self.backend.get_stats().await
    }

    /// Clean up old data based on retention policy
    pub async fn cleanup_old_data(&self, retention_days: u32) -> Result<u64> {
        self.backend.cleanup_old_data(retention_days).await
    }
}

/// Encrypted note stored in the client database
#[derive(Debug, Clone)]
pub struct EncryptedNote {
    pub note_id: NoteId,
    pub tag: NoteTag,
    pub header: NoteHeader,
    pub encrypted_data: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub stored_at: DateTime<Utc>,
}

/// Client database statistics
#[derive(Debug, Clone)]
pub struct ClientDatabaseStats {
    pub public_keys_count: u64,
    pub fetched_notes_count: u64,
    pub encrypted_notes_count: u64,
    pub unique_tags_count: u64,
}

#[cfg(test)]
mod tests {
    use miden_objects::testing::account_id::ACCOUNT_ID_MAX_ZEROES;

    use super::*;
    use crate::types::random_note_id;

    #[tokio::test]
    async fn test_client_database_operations() {
        let config = ClientDatabaseConfig {
            database_path: ":memory:".to_string(),
            ..Default::default()
        };

        let db = ClientDatabase::new_sqlite(config).await.unwrap();

        // Test public key storage
        let account_id = AccountId::try_from(ACCOUNT_ID_MAX_ZEROES).unwrap();
        let key = SerializableKey::generate_aes();

        db.store_key(&account_id, &key).await.unwrap();

        let retrieved_key = db.get_key(&account_id).await.unwrap();
        assert!(retrieved_key.is_some());
        // Note: We can't compare keys directly with to_string() since SerializableKey doesn't
        // implement Display Instead, we verify the key was stored and retrieved
        // successfully

        // Test fetched note recording
        let note_id = random_note_id();
        let tag = NoteTag::from(123);

        db.record_fetched_note(&note_id, tag).await.unwrap();

        // Test encrypted note storage
        let header = crate::types::test_note_header();
        let encrypted_data = vec![1, 2, 3, 4];
        let created_at = Utc::now();

        db.store_encrypted_note(&note_id, tag, &header, &encrypted_data, created_at)
            .await
            .unwrap();

        let stored_note = db.get_encrypted_note(&note_id).await.unwrap();
        assert!(stored_note.is_some());

        let stored_note = stored_note.unwrap();
        assert_eq!(stored_note.note_id, note_id);
        assert_eq!(stored_note.tag, tag);
        assert_eq!(stored_note.encrypted_data, encrypted_data);

        // Test statistics
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.public_keys_count, 1);
        assert_eq!(stats.fetched_notes_count, 1);
        assert_eq!(stats.encrypted_notes_count, 1);
        assert_eq!(stats.unique_tags_count, 1);
    }
}
