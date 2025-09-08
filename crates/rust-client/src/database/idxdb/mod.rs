use alloc::{boxed::Box, vec::Vec};

use chrono::{DateTime, Utc};
use miden_objects::note::{NoteHeader, NoteId, NoteTag};

use crate::database::{DatabaseBackend, DatabaseError, DatabaseStats, StoredNote};

pub struct IndexedDb;

impl IndexedDb {
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(not(feature = "idxdb"), async_trait::async_trait)]
#[cfg_attr(feature = "idxdb", async_trait::async_trait(?Send))]
impl DatabaseBackend for IndexedDb {
    /// Store a note
    async fn store_note(
        &self,
        _header: &NoteHeader,
        _encrypted_data: &[u8],
        _created_at: DateTime<Utc>,
    ) -> Result<(), DatabaseError> {
        Ok(())
    }

    /// Get an stored note by ID
    async fn get_stored_note(
        &self,
        _note_id: &NoteId,
    ) -> Result<Option<StoredNote>, DatabaseError> {
        Ok(None)
    }

    /// Get all stored notes for a tag
    async fn get_stored_notes_for_tag(
        &self,
        _tag: NoteTag,
    ) -> Result<Vec<StoredNote>, DatabaseError> {
        Ok(vec![])
    }

    /// Record that a note has been fetched
    async fn record_fetched_note(
        &self,
        _note_id: &NoteId,
        _tag: NoteTag,
    ) -> Result<(), DatabaseError> {
        Ok(())
    }

    /// Check if a note has been fetched before
    async fn note_fetched(&self, _note_id: &NoteId) -> Result<bool, DatabaseError> {
        Ok(false)
    }

    /// Get all fetched note IDs for a specific tag
    async fn get_fetched_notes_for_tag(&self, _tag: NoteTag) -> Result<Vec<NoteId>, DatabaseError> {
        Ok(vec![])
    }

    /// Get database statistics
    async fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        Ok(DatabaseStats {
            fetched_notes_count: 0,
            stored_notes_count: 0,
            unique_tags_count: 0,
        })
    }

    /// Clean up old data based on retention policy
    async fn cleanup_old_data(&self, _retention_days: u32) -> Result<u64, DatabaseError> {
        Ok(0)
    }
}
