use super::{DatabaseBackend, DatabaseConfig};
use crate::{
    types::{EncryptedDetails, NoteHeader, NoteId, NoteTag, StoredNote},
    Error, Result,
};
use chrono::{DateTime, Utc};
use miden_objects::utils::{Deserializable, Serializable};
use sqlx::{sqlite::SqlitePool, Row};

/// SQLite implementation of the database backend
pub struct SQLiteDB {
    pool: SqlitePool,
}

#[async_trait::async_trait]
impl DatabaseBackend for SQLiteDB {
    async fn connect(config: DatabaseConfig) -> Result<Self> {
        let pool = SqlitePool::connect(&config.url).await?;

        // Create tables if they don't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS notes (
                id BLOB PRIMARY KEY,
                tag INTEGER NOT NULL,
                header BLOB NOT NULL,
                encrypted_data BLOB NOT NULL,
                created_at TEXT NOT NULL,
                received_by TEXT
            ) STRICT;
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_notes_tag ON notes(tag);
            CREATE INDEX IF NOT EXISTS idx_notes_created_at ON notes(created_at);
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    async fn store_note(&self, note: &StoredNote) -> Result<()> {
        let received_by_json = if let Some(ref received_by) = note.received_by {
            serde_json::to_string(received_by)?
        } else {
            "[]".to_string()
        };

        sqlx::query(
            r#"
            INSERT INTO notes (id, tag, header, encrypted_data, created_at, received_by)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&note.header.id().inner().as_bytes()[..])
        .bind(note.header.metadata().tag().as_u32() as i64)
        .bind(note.header.to_bytes())
        .bind(&note.encrypted_data.0)
        .bind(note.created_at.to_rfc3339())
        .bind(received_by_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn fetch_notes(&self, tag: NoteTag) -> Result<Vec<StoredNote>> {
        let query = sqlx::query(
            r#"
                SELECT id, tag, header, encrypted_data, created_at, received_by
                FROM notes
                WHERE tag = ?
                ORDER BY created_at ASC
                "#,
        )
        .bind(tag.as_u32() as i64);

        let rows = query.fetch_all(&self.pool).await?;
        let mut notes = Vec::new();

        for row in rows {
            let _id_bytes: Vec<u8> = row.try_get("id")?;
            let _tag: i64 = row.try_get("tag")?;
            let header_bytes: Vec<u8> = row.try_get("header")?;
            let encrypted_data: Vec<u8> = row.try_get("encrypted_data")?;
            let created_at_str: String = row.try_get("created_at")?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| {
                    Error::Database(sqlx::Error::ColumnDecode {
                        index: "created_at".to_string(),
                        source: Box::new(e),
                    })
                })?
                .with_timezone(&Utc);

            let received_by_json: String = row.try_get("received_by")?;

            let received_by: Option<Vec<String>> = if received_by_json == "[]" {
                None
            } else {
                Some(serde_json::from_str(&received_by_json)?)
            };

            let header = NoteHeader::read_from_bytes(&header_bytes).map_err(|e| {
                Error::Database(sqlx::Error::ColumnDecode {
                    index: "header".to_string(),
                    source: Box::new(e),
                })
            })?;

            let note = StoredNote {
                header,
                encrypted_data: EncryptedDetails(encrypted_data),
                created_at,
                received_by,
            };

            notes.push(note);
        }

        Ok(notes)
    }

    async fn mark_received(&self, note_id: NoteId, user_id: &str) -> Result<()> {
        // First, get the current received_by list
        let row = sqlx::query(
            r#"
            SELECT received_by FROM notes WHERE id = ?
            "#,
        )
        .bind(&note_id.inner().as_bytes()[..])
        .fetch_one(&self.pool)
        .await?;

        let received_by_json: String = row.try_get("received_by")?;
        let mut received_by: Vec<String> = if received_by_json == "[]" {
            Vec::new()
        } else {
            serde_json::from_str(&received_by_json)?
        };

        // Add the user if not already present
        if !received_by.contains(&user_id.to_string()) {
            received_by.push(user_id.to_string());
        }

        let updated_json = serde_json::to_string(&received_by)?;

        sqlx::query(
            r#"
            UPDATE notes SET received_by = ? WHERE id = ?
            "#,
        )
        .bind(updated_json)
        .bind(&note_id.inner().as_bytes()[..])
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_stats(&self) -> Result<(u64, u64)> {
        let total_notes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes")
            .fetch_one(&self.pool)
            .await?;

        let total_tags: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT tag) FROM notes")
            .fetch_one(&self.pool)
            .await?;

        Ok((total_notes as u64, total_tags as u64))
    }

    async fn cleanup_old_notes(&self, retention_days: u32) -> Result<u64> {
        let cutoff_date = Utc::now() - chrono::Duration::days(retention_days as i64);

        let result = sqlx::query(
            r#"
            DELETE FROM notes WHERE created_at < ?
            "#,
        )
        .bind(cutoff_date.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn note_exists(&self, note_id: NoteId) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM notes WHERE id = ?
            "#,
        )
        .bind(&note_id.inner().as_bytes()[..])
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }
}
