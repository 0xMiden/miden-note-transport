use chrono::{DateTime, Utc};
use miden_objects::{
    account::AccountId,
    note::{NoteHeader, NoteId, NoteTag},
    utils::{Deserializable, Serializable},
};
use sqlx::{Row, SqlitePool};

use super::{ClientDatabaseBackend, ClientDatabaseConfig, ClientDatabaseStats, EncryptedNote};
use crate::Result;

/// SQLite implementation of the client database
pub struct SqliteClientDatabase {
    pool: SqlitePool,
}

impl SqliteClientDatabase {
    /// Connect to the SQLite client database
    pub async fn connect(config: ClientDatabaseConfig) -> Result<Self> {
        let pool = SqlitePool::connect(&format!("sqlite:{}", config.database_path)).await?;

        // Create tables if they don't exist
        Self::create_tables(&pool).await?;

        Ok(Self { pool })
    }

    /// Create all necessary tables
    async fn create_tables(pool: &SqlitePool) -> Result<()> {
        // Table for storing public keys associated with account IDs
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS public_keys (
                account_id BLOB PRIMARY KEY,
                key_data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            ) STRICT;
            ",
        )
        .execute(pool)
        .await?;

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

        // Table for storing encrypted notes
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS encrypted_notes (
                note_id BLOB PRIMARY KEY,
                tag INTEGER NOT NULL,
                header BLOB NOT NULL,
                encrypted_data BLOB NOT NULL,
                created_at TEXT NOT NULL,
                stored_at TEXT NOT NULL
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
            CREATE INDEX IF NOT EXISTS idx_encrypted_notes_tag ON encrypted_notes(tag);
            CREATE INDEX IF NOT EXISTS idx_encrypted_notes_created_at ON encrypted_notes(created_at);
            ",
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl ClientDatabaseBackend for SqliteClientDatabase {
    async fn store_key(
        &self,
        account_id: &AccountId,
        key: &crate::client::crypto::SerializableKey,
    ) -> Result<()> {
        let now = Utc::now();
        let key_json = serde_json::to_string(key)?;

        sqlx::query(
            r"
            INSERT OR REPLACE INTO public_keys (account_id, key_data, created_at, updated_at)
            VALUES (?, ?, ?, ?)
            ",
        )
        .bind(&account_id.to_bytes()[..])
        .bind(&key_json)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_key(
        &self,
        account_id: &AccountId,
    ) -> Result<Option<crate::client::crypto::SerializableKey>> {
        let row = sqlx::query(
            r"
            SELECT key_data FROM public_keys WHERE account_id = ?
            ",
        )
        .bind(&account_id.to_bytes()[..])
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let key_json: String = row.try_get("key_data")?;
            let key: crate::client::crypto::SerializableKey = serde_json::from_str(&key_json)?;
            Ok(Some(key))
        } else {
            Ok(None)
        }
    }

    async fn get_all_keys(
        &self,
    ) -> Result<Vec<(AccountId, crate::client::crypto::SerializableKey)>> {
        let rows = sqlx::query(
            r"
            SELECT account_id, key_data FROM public_keys
            ORDER BY created_at ASC
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut keys = Vec::new();
        for row in rows {
            let account_id_bytes: Vec<u8> = row.try_get("account_id")?;
            let key_json: String = row.try_get("key_data")?;

            let account_id = AccountId::read_from_bytes(&account_id_bytes).map_err(|e| {
                crate::Error::Database(sqlx::Error::ColumnDecode {
                    index: "account_id".to_string(),
                    source: Box::new(e),
                })
            })?;
            let key: crate::client::crypto::SerializableKey = serde_json::from_str(&key_json)?;
            keys.push((account_id, key));
        }

        Ok(keys)
    }

    async fn store_encrypted_note(
        &self,
        note_id: &NoteId,
        tag: NoteTag,
        header: &NoteHeader,
        encrypted_data: &[u8],
        created_at: DateTime<Utc>,
    ) -> Result<()> {
        let now = Utc::now();
        let header_bytes = header.to_bytes();

        sqlx::query(
            r"
            INSERT OR REPLACE INTO encrypted_notes (note_id, tag, header, encrypted_data, created_at, stored_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&note_id.inner().as_bytes()[..])
        .bind(i64::from(tag.as_u32()))
        .bind(&header_bytes)
        .bind(encrypted_data)
        .bind(created_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_encrypted_note(&self, note_id: &NoteId) -> Result<Option<EncryptedNote>> {
        let row = sqlx::query(
            r"
            SELECT tag, header, encrypted_data, created_at, stored_at
            FROM encrypted_notes WHERE note_id = ?
            ",
        )
        .bind(&note_id.inner().as_bytes()[..])
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let tag_value: i64 = row.try_get("tag")?;
            let header_bytes: Vec<u8> = row.try_get("header")?;
            let encrypted_data: Vec<u8> = row.try_get("encrypted_data")?;
            let created_at_str: String = row.try_get("created_at")?;
            let stored_at_str: String = row.try_get("stored_at")?;

            let tag = NoteTag::from(u32::try_from(tag_value).map_err(|e| {
                crate::Error::Database(sqlx::Error::ColumnDecode {
                    index: "tag".to_string(),
                    source: Box::new(e),
                })
            })?);
            let header = NoteHeader::read_from_bytes(&header_bytes).map_err(|e| {
                crate::Error::Database(sqlx::Error::ColumnDecode {
                    index: "header".to_string(),
                    source: Box::new(e),
                })
            })?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| {
                    crate::Error::Database(sqlx::Error::ColumnDecode {
                        index: "created_at".to_string(),
                        source: Box::new(e),
                    })
                })?
                .with_timezone(&Utc);
            let stored_at = DateTime::parse_from_rfc3339(&stored_at_str)
                .map_err(|e| {
                    crate::Error::Database(sqlx::Error::ColumnDecode {
                        index: "stored_at".to_string(),
                        source: Box::new(e),
                    })
                })?
                .with_timezone(&Utc);

            Ok(Some(EncryptedNote {
                note_id: *note_id,
                tag,
                header,
                encrypted_data,
                created_at,
                stored_at,
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_encrypted_notes_for_tag(&self, tag: NoteTag) -> Result<Vec<EncryptedNote>> {
        let rows = sqlx::query(
            r"
            SELECT note_id, header, encrypted_data, created_at, stored_at
            FROM encrypted_notes WHERE tag = ?
            ORDER BY created_at ASC
            ",
        )
        .bind(i64::from(tag.as_u32()))
        .fetch_all(&self.pool)
        .await?;

        let mut notes = Vec::new();
        for row in rows {
            let note_id_bytes: Vec<u8> = row.try_get("note_id")?;
            let header_bytes: Vec<u8> = row.try_get("header")?;
            let encrypted_data: Vec<u8> = row.try_get("encrypted_data")?;
            let created_at_str: String = row.try_get("created_at")?;
            let stored_at_str: String = row.try_get("stored_at")?;

            let note_id = NoteId::read_from_bytes(&note_id_bytes).map_err(|e| {
                crate::Error::Database(sqlx::Error::ColumnDecode {
                    index: "note_id".to_string(),
                    source: Box::new(e),
                })
            })?;
            let header = NoteHeader::read_from_bytes(&header_bytes).map_err(|e| {
                crate::Error::Database(sqlx::Error::ColumnDecode {
                    index: "header".to_string(),
                    source: Box::new(e),
                })
            })?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| {
                    crate::Error::Database(sqlx::Error::ColumnDecode {
                        index: "created_at".to_string(),
                        source: Box::new(e),
                    })
                })?
                .with_timezone(&Utc);
            let stored_at = DateTime::parse_from_rfc3339(&stored_at_str)
                .map_err(|e| {
                    crate::Error::Database(sqlx::Error::ColumnDecode {
                        index: "stored_at".to_string(),
                        source: Box::new(e),
                    })
                })?
                .with_timezone(&Utc);

            notes.push(EncryptedNote {
                note_id,
                tag,
                header,
                encrypted_data,
                created_at,
                stored_at,
            });
        }

        Ok(notes)
    }

    async fn record_fetched_note(&self, note_id: &NoteId, tag: NoteTag) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            r"
            INSERT OR REPLACE INTO fetched_notes (note_id, tag, fetched_at)
            VALUES (?, ?, ?)
            ",
        )
        .bind(&note_id.inner().as_bytes()[..])
        .bind(i64::from(tag.as_u32()))
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn note_fetched(&self, note_id: &NoteId) -> Result<bool> {
        let row = sqlx::query(
            r"
            SELECT 1 FROM fetched_notes WHERE note_id = ?
            ",
        )
        .bind(&note_id.inner().as_bytes()[..])
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.is_some())
    }

    async fn get_fetched_notes_for_tag(&self, tag: NoteTag) -> Result<Vec<NoteId>> {
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
            let note_id = NoteId::read_from_bytes(&note_id_bytes).map_err(|e| {
                crate::Error::Database(sqlx::Error::ColumnDecode {
                    index: "note_id".to_string(),
                    source: Box::new(e),
                })
            })?;
            note_ids.push(note_id);
        }

        Ok(note_ids)
    }

    async fn get_stats(&self) -> Result<ClientDatabaseStats> {
        let public_keys_count: u64 = sqlx::query_scalar("SELECT COUNT(*) FROM public_keys")
            .fetch_one(&self.pool)
            .await?;

        let fetched_notes_count: u64 = sqlx::query_scalar("SELECT COUNT(*) FROM fetched_notes")
            .fetch_one(&self.pool)
            .await?;

        let encrypted_notes_count: u64 = sqlx::query_scalar("SELECT COUNT(*) FROM encrypted_notes")
            .fetch_one(&self.pool)
            .await?;

        let unique_tags_count: u64 =
            sqlx::query_scalar("SELECT COUNT(DISTINCT tag) FROM encrypted_notes")
                .fetch_one(&self.pool)
                .await?;

        Ok(ClientDatabaseStats {
            public_keys_count: public_keys_count as u64,
            fetched_notes_count: fetched_notes_count as u64,
            encrypted_notes_count: encrypted_notes_count as u64,
            unique_tags_count: unique_tags_count as u64,
        })
    }

    async fn cleanup_old_data(&self, retention_days: u32) -> Result<u64> {
        let cutoff_date = Utc::now() - chrono::Duration::days(i64::from(retention_days));

        let result = sqlx::query(
            r"
            DELETE FROM encrypted_notes WHERE created_at < ?
            ",
        )
        .bind(cutoff_date.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
