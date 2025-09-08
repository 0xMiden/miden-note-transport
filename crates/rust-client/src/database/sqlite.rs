use std::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

use chrono::{DateTime, Utc};
use miden_objects::{
    note::{NoteHeader, NoteId, NoteTag},
    utils::{Deserializable, Serializable},
};
use sqlx::{Row, SqlitePool};

use super::{DatabaseBackend, DatabaseConfig, DatabaseError, DatabaseStats, StoredNote};

/// `SQLite` implementation of the client database
pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    /// Connect to the `SQLite` client database
    pub async fn connect(config: DatabaseConfig) -> Result<Self, DatabaseError> {
        if !std::path::Path::new(&config.url).exists() && !config.url.contains(":memory:") {
            std::fs::File::create(&config.url).map_err(anyhow::Error::new)?;
        }
        let url = format!("sqlite:{}", config.url);

        let pool = SqlitePool::connect(&url).await?;

        // Create tables if they don't exist
        Self::create_tables(&pool).await?;

        Ok(Self { pool })
    }

    /// Create all necessary tables
    async fn create_tables(pool: &SqlitePool) -> Result<(), DatabaseError> {
        // Table for storing fetched note IDs
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS fetched_notes (
                note_id BLOB PRIMARY KEY,
                tag INTEGER NOT NULL,
                fetched_at TEXT NOT NULL
            ) STRICT;
            ",
        )
        .execute(pool)
        .await?;

        // Table for storing notes
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS stored_notes (
                note_id BLOB PRIMARY KEY,
                tag INTEGER NOT NULL,
                header BLOB NOT NULL,
                details BLOB NOT NULL,
                created_at TEXT NOT NULL
            ) STRICT;
            ",
        )
        .execute(pool)
        .await?;

        // Create indexes for better performance
        sqlx::query(
            r"
            CREATE INDEX IF NOT EXISTS idx_fetched_notes_tag ON fetched_notes(tag);
            CREATE INDEX IF NOT EXISTS idx_fetched_notes_fetched_at ON fetched_notes(fetched_at);
            CREATE INDEX IF NOT EXISTS idx_stored_notes_tag ON stored_notes(tag);
            CREATE INDEX IF NOT EXISTS idx_stored_notes_created_at ON stored_notes(created_at);
            ",
        )
        .execute(pool)
        .await?;

        Ok(())
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
        let note_id = header.id();
        let tag = header.metadata().tag();
        let header_bytes = header.to_bytes();

        sqlx::query(
            r"
            INSERT OR REPLACE INTO stored_notes (note_id, tag, header, details, created_at)
            VALUES (?, ?, ?, ?, ?)
            ",
        )
        .bind(&note_id.as_bytes()[..])
        .bind(i64::from(tag.as_u32()))
        .bind(&header_bytes)
        .bind(details)
        .bind(created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_stored_note(&self, note_id: &NoteId) -> Result<Option<StoredNote>, DatabaseError> {
        let row = sqlx::query(
            r"
            SELECT tag, header, details, created_at
            FROM stored_notes WHERE note_id = ?
            ",
        )
        .bind(&note_id.as_bytes()[..])
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let header_bytes: Vec<u8> = row.try_get("header")?;
            let details: Vec<u8> = row.try_get("details")?;
            let created_at_str: String = row.try_get("created_at")?;

            let header = NoteHeader::read_from_bytes(&header_bytes)
                .map_err(|e| DatabaseError::Encoding(e.to_string()))?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| DatabaseError::Encoding(e.to_string()))?
                .with_timezone(&Utc);

            Ok(Some(StoredNote { header, details, created_at }))
        } else {
            Ok(None)
        }
    }

    async fn get_stored_notes_for_tag(
        &self,
        tag: NoteTag,
    ) -> Result<Vec<StoredNote>, DatabaseError> {
        let rows = sqlx::query(
            r"
            SELECT note_id, header, details, created_at
            FROM stored_notes WHERE tag = ?
            ORDER BY created_at ASC
            ",
        )
        .bind(i64::from(tag.as_u32()))
        .fetch_all(&self.pool)
        .await?;

        let mut notes = Vec::new();
        for row in rows {
            let header_bytes: Vec<u8> = row.try_get("header")?;
            let details: Vec<u8> = row.try_get("details")?;
            let created_at_str: String = row.try_get("created_at")?;

            let header = NoteHeader::read_from_bytes(&header_bytes)
                .map_err(|e| DatabaseError::Encoding(e.to_string()))?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| DatabaseError::Encoding(e.to_string()))?
                .with_timezone(&Utc);

            notes.push(StoredNote { header, details, created_at });
        }

        Ok(notes)
    }

    async fn record_fetched_note(
        &self,
        note_id: &NoteId,
        tag: NoteTag,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();

        sqlx::query(
            r"
            INSERT OR REPLACE INTO fetched_notes (note_id, tag, fetched_at)
            VALUES (?, ?, ?)
            ",
        )
        .bind(&note_id.as_bytes()[..])
        .bind(i64::from(tag.as_u32()))
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn note_fetched(&self, note_id: &NoteId) -> Result<bool, DatabaseError> {
        let row = sqlx::query(
            r"
            SELECT 1 FROM fetched_notes WHERE note_id = ?
            ",
        )
        .bind(&note_id.as_bytes()[..])
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.is_some())
    }

    async fn get_fetched_notes_for_tag(&self, tag: NoteTag) -> Result<Vec<NoteId>, DatabaseError> {
        let rows = sqlx::query(
            r"
            SELECT note_id FROM fetched_notes WHERE tag = ?
            ORDER BY fetched_at ASC
            ",
        )
        .bind(i64::from(tag.as_u32()))
        .fetch_all(&self.pool)
        .await?;

        let mut note_ids = Vec::new();
        for row in rows {
            let note_id_bytes: Vec<u8> = row.try_get("note_id")?;
            let note_id = NoteId::read_from_bytes(&note_id_bytes)
                .map_err(|e| DatabaseError::Encoding(e.to_string()))?;
            note_ids.push(note_id);
        }

        Ok(note_ids)
    }

    async fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        let fetched_notes_count: u64 = sqlx::query_scalar("SELECT COUNT(*) FROM fetched_notes")
            .fetch_one(&self.pool)
            .await?;

        let stored_notes_count: u64 = sqlx::query_scalar("SELECT COUNT(*) FROM stored_notes")
            .fetch_one(&self.pool)
            .await?;

        let unique_tags_count: u64 =
            sqlx::query_scalar("SELECT COUNT(DISTINCT tag) FROM stored_notes")
                .fetch_one(&self.pool)
                .await?;

        Ok(DatabaseStats {
            fetched_notes_count: fetched_notes_count as u64,
            stored_notes_count: stored_notes_count as u64,
            unique_tags_count: unique_tags_count as u64,
        })
    }

    async fn cleanup_old_data(&self, retention_days: u32) -> Result<u64, DatabaseError> {
        let cutoff_date = Utc::now() - chrono::Duration::days(i64::from(retention_days));

        let result = sqlx::query(
            r"
            DELETE FROM stored_notes WHERE created_at < ?
            ",
        )
        .bind(cutoff_date.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

impl From<sqlx::Error> for DatabaseError {
    fn from(se: sqlx::Error) -> Self {
        match se {
            sqlx::Error::Configuration(e) => Self::Configuration(e.to_string()),
            sqlx::Error::Protocol(e) => Self::Protocol(e.to_string()),
            sqlx::Error::RowNotFound => Self::NotFound("Row not found".to_string()),
            sqlx::Error::TypeNotFound { type_name } => Self::NotFound(type_name),
            sqlx::Error::ColumnNotFound(e) => Self::NotFound(e),
            e => anyhow::Error::new(e).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use miden_objects::{note::NoteDetails, utils::Serializable};

    use super::{super::Database, *};
    use crate::types::mock_note_p2id;

    #[tokio::test]
    async fn test_client_database_sqlite_operations() {
        let config = DatabaseConfig::default();

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
