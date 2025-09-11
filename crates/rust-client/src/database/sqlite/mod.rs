//! This module provides an SQLite-backed implementation of the [`DatabaseBackend`] trait.
//!
//! [`SqliteDatabase`] enables the persistence of notes and metadata using an `SQLite` database.
//! It is compiled only when the `sqlite` feature flag is enabled.

mod db_management;
mod note;
mod stats;

use alloc::{boxed::Box, string::ToString, vec::Vec};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use miden_objects::note::{NoteHeader, NoteId, NoteTag};
use rusqlite::Connection;

use super::{DatabaseBackend, DatabaseConfig, DatabaseError, DatabaseStats, StoredNote};
use crate::database::sqlite::{
    db_management::{
        pool_manager::{Pool, SqlitePoolManager},
        utils::apply_migrations,
    },
    note::NoteOperations,
    stats::StatsOperations,
};

/// `SQLite` implementation of the client database
pub struct SqliteDatabase {
    pool: Pool,
}

impl SqliteDatabase {
    /// Connect to the `SQLite` client database
    pub async fn connect(config: DatabaseConfig) -> Result<Self, DatabaseError> {
        if !std::path::Path::new(&config.url).exists() && !config.url.contains(":memory:") {
            std::fs::File::create(&config.url).map_err(anyhow::Error::new)?;
        }

        let database_path = PathBuf::from(&config.url);
        let sqlite_pool_manager = SqlitePoolManager::new(database_path);
        let pool = Pool::builder(sqlite_pool_manager)
            .build()
            .map_err(|e| DatabaseError::Configuration(e.to_string()))?;

        let conn = pool.get().await.map_err(|e| DatabaseError::Configuration(e.to_string()))?;

        conn.interact(apply_migrations)
            .await
            .map_err(|e| DatabaseError::Configuration(e.to_string()))?
            .map_err(|e| DatabaseError::Configuration(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Interacts with the database by executing the provided function on a connection from the
    /// pool.
    ///
    /// This function is a helper method which simplifies the process of making queries to the
    /// database. It acquires a connection from the pool and executes the provided function,
    /// returning the result.
    async fn interact_with_connection<F, R>(&self, f: F) -> Result<R, DatabaseError>
    where
        F: FnOnce(&mut Connection) -> Result<R, DatabaseError> + Send + 'static,
        R: Send + 'static,
    {
        self.pool
            .get()
            .await
            .map_err(|err| DatabaseError::Configuration(err.to_string()))?
            .interact(f)
            .await
            .map_err(|err| DatabaseError::Configuration(err.to_string()))?
    }
}

#[async_trait::async_trait]
impl DatabaseBackend for SqliteDatabase {
    async fn store_note(
        &self,
        header: &NoteHeader,
        details: &[u8],
        created_at: DateTime<Utc>,
    ) -> Result<(), DatabaseError> {
        let header = *header;
        let details = details.to_vec();
        self.interact_with_connection(move |conn| {
            NoteOperations::store_note(conn, &header, &details, created_at)
        })
        .await
    }

    async fn get_stored_note(&self, note_id: &NoteId) -> Result<Option<StoredNote>, DatabaseError> {
        let note_id = *note_id;
        self.interact_with_connection(move |conn| NoteOperations::get_stored_note(conn, &note_id))
            .await
    }

    async fn get_stored_notes_for_tag(
        &self,
        tag: NoteTag,
    ) -> Result<Vec<StoredNote>, DatabaseError> {
        self.interact_with_connection(move |conn| {
            NoteOperations::get_stored_notes_for_tag(conn, tag)
        })
        .await
    }

    async fn record_fetched_note(
        &self,
        note_id: &NoteId,
        tag: NoteTag,
    ) -> Result<(), DatabaseError> {
        let note_id = *note_id;
        self.interact_with_connection(move |conn| {
            NoteOperations::record_fetched_note(conn, &note_id, tag)
        })
        .await
    }

    async fn note_fetched(&self, note_id: &NoteId) -> Result<bool, DatabaseError> {
        let note_id = *note_id;
        self.interact_with_connection(move |conn| NoteOperations::note_fetched(conn, &note_id))
            .await
    }

    async fn get_fetched_notes_for_tag(&self, tag: NoteTag) -> Result<Vec<NoteId>, DatabaseError> {
        self.interact_with_connection(move |conn| {
            NoteOperations::get_fetched_notes_for_tag(conn, tag)
        })
        .await
    }

    async fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        self.interact_with_connection(StatsOperations::get_stats).await
    }

    async fn cleanup_old_data(&self, retention_days: u32) -> Result<u64, DatabaseError> {
        self.interact_with_connection(move |conn| {
            StatsOperations::cleanup_old_data(conn, retention_days)
        })
        .await
    }
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(se: rusqlite::Error) -> Self {
        match se {
            rusqlite::Error::InvalidColumnType(..) => {
                Self::Configuration(format!("Invalid column type: {se}"))
            },
            rusqlite::Error::InvalidColumnIndex(_) => {
                Self::Configuration(format!("Invalid column index: {se}"))
            },
            rusqlite::Error::InvalidParameterCount(..) => {
                Self::Configuration(format!("Invalid parameter count: {se}"))
            },
            rusqlite::Error::InvalidColumnName(_) => {
                Self::NotFound(format!("Column not found: {se}"))
            },
            rusqlite::Error::SqliteFailure(..) => Self::Protocol(format!("SQLite error: {se}")),
            e => anyhow::Error::new(e).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use miden_objects::{note::NoteDetails, utils::Serializable};

    use super::{super::Database, *};
    use crate::test_utils::mock_note_p2id;

    #[tokio::test]
    async fn test_client_database_sqlite_operations() {
        // Use in-memory SQLite database for testing
        let config = DatabaseConfig {
            url: ":memory:".to_string(),
            max_note_size: 1024 * 1024,
        };

        let db = Database::new_sqlite(config).await.unwrap();

        let note = mock_note_p2id();
        let note_id = note.id();
        let tag = note.metadata().tag();
        let header = *note.header();
        let details = NoteDetails::from(note).to_bytes();

        db.record_fetched_note(&note_id, tag).await.unwrap();

        let created_at = Utc::now();
        db.store_note(&header, &details, created_at).await.unwrap();

        let stored_note = db.get_stored_note(&note_id).await.unwrap();
        assert!(stored_note.is_some());

        let stored_note = stored_note.unwrap();
        assert_eq!(stored_note.header.id(), note_id);
        assert_eq!(stored_note.details, details);

        // Test statistics
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.fetched_notes_count, 1);
        assert_eq!(stats.stored_notes_count, 1);
        assert_eq!(stats.unique_tags_count, 1);
    }
}
