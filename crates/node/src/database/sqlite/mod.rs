use chrono::Utc;
use deadpool_diesel::sqlite::{Manager, Pool};
use diesel::prelude::*;

use crate::{
    database::{DatabaseBackend, DatabaseConfig, DatabaseError},
    metrics::MetricsDatabase,
    types::{NoteId, NoteTag, StoredNote},
};

mod migrations;
mod models;
mod schema;

use models::{NewNote, Note};

/// `SQLite` implementation of the database backend
pub struct SqliteDatabase {
    pool: Pool,
    metrics: MetricsDatabase,
}

impl SqliteDatabase {
    /// Get a connection from the pool
    async fn get_connection(
        &self,
    ) -> std::result::Result<deadpool::managed::Object<Manager>, DatabaseError> {
        self.pool
            .get()
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to get connection: {e}")))
    }

    /// Execute a query
    async fn execute_query<F, R>(
        &self,
        operation: &str,
        query: F,
    ) -> std::result::Result<R, DatabaseError>
    where
        F: FnOnce(&mut SqliteConnection) -> std::result::Result<R, diesel::result::Error>
            + Send
            + 'static,
        R: Send + 'static,
    {
        let conn = self.get_connection().await?;

        let query_with_db_error =
            move |conn: &mut SqliteConnection| -> std::result::Result<R, DatabaseError> {
                query(conn)
                    .map_err(|e| DatabaseError::QueryExecution(format!("Database error: {e}")))
            };

        conn.interact(query_with_db_error)
            .await
            .map_err(|e| DatabaseError::QueryExecution(format!("Failed to {operation}: {e}")))?
    }
}

#[async_trait::async_trait]
impl DatabaseBackend for SqliteDatabase {
    async fn connect(
        config: DatabaseConfig,
        metrics: MetricsDatabase,
    ) -> Result<Self, DatabaseError> {
        if !std::path::Path::new(&config.url).exists() && !config.url.contains(":memory:") {
            std::fs::File::create(&config.url).map_err(|e| {
                DatabaseError::Configuration(format!("Failed to create database file: {e}"))
            })?;
        }

        let manager = Manager::new(config.url, deadpool_diesel::Runtime::Tokio1);
        let pool = Pool::builder(manager)
            .build()
            .map_err(|e| DatabaseError::Pool(format!("Failed to create connection pool: {e}")))?;

        // Apply migrations
        let conn = pool
            .get()
            .await
            .map_err(|e| DatabaseError::Connection(format!("Failed to get connection: {e}")))?;
        tracing::debug!("Applying migrations to database");
        conn.interact(migrations::apply_migrations)
            .await
            .map_err(|e| DatabaseError::Migration(format!("Failed to apply migrations: {e}")))?
            .map_err(|e| DatabaseError::Migration(format!("Migration error: {e}")))?;

        Ok(Self { pool, metrics })
    }

    #[tracing::instrument(skip(self), fields(operation = "db.store_note"))]
    async fn store_note(&self, note: &StoredNote) -> Result<(), DatabaseError> {
        let timer = self.metrics.db_store_note();

        let new_note = NewNote::from(note);
        self.execute_query("store note", move |conn| {
            diesel::insert_into(schema::notes::table).values(&new_note).execute(conn)?;
            Ok(())
        })
        .await?;

        timer.finish("ok");

        Ok(())
    }

    #[tracing::instrument(skip(self), fields(operation = "db.fetch_notes"))]
    async fn fetch_notes(
        &self,
        tag: NoteTag,
        cursor: u64,
    ) -> Result<Vec<StoredNote>, DatabaseError> {
        let timer = self.metrics.db_fetch_notes();

        let cursor_i64: i64 = cursor.try_into().map_err(|_| {
            DatabaseError::QueryExecution("Cursor too large for SQLite".to_string())
        })?;

        let tag_value = i64::from(tag.as_u32());
        let notes: Vec<Note> = self
            .execute_query("fetch notes", move |conn| {
                use schema::notes::dsl::{created_at, notes, tag};
                let fetched_notes = notes
                    .filter(tag.eq(tag_value))
                    .filter(created_at.gt(cursor_i64))
                    .order(created_at.asc())
                    .load::<Note>(conn)?;
                Ok(fetched_notes)
            })
            .await?;

        let mut stored_notes = Vec::new();
        for note in notes {
            let stored_note = StoredNote::try_from(note).map_err(|e| {
                DatabaseError::Deserialization(format!("Failed to deserialize note: {e}"))
            })?;
            stored_notes.push(stored_note);
        }

        timer.finish("ok");

        Ok(stored_notes)
    }

    async fn get_stats(&self) -> Result<(u64, u64), DatabaseError> {
        let (total_notes, total_tags): (i64, i64) = self
            .execute_query("get stats", |conn| {
                #[allow(deprecated)]
                use diesel::dsl::count_distinct;
                use schema::notes::dsl::{notes, tag};

                let total_notes: i64 = notes.count().get_result(conn)?;
                #[allow(deprecated)]
                let total_tags: i64 = notes.select(count_distinct(tag)).first(conn)?;

                Ok((total_notes, total_tags))
            })
            .await?;

        Ok((total_notes.try_into().unwrap_or(0), total_tags.try_into().unwrap_or(0)))
    }

    async fn cleanup_old_notes(&self, retention_days: u32) -> Result<u64, DatabaseError> {
        let cutoff_date = Utc::now() - chrono::Duration::days(i64::from(retention_days));
        let cutoff_timestamp = cutoff_date.timestamp_micros();

        let deleted_count: i64 = self
            .execute_query("cleanup old notes", move |conn| {
                use schema::notes::dsl::{created_at, notes};
                let count =
                    diesel::delete(notes.filter(created_at.lt(cutoff_timestamp))).execute(conn)?;
                Ok(i64::try_from(count).unwrap_or(0))
            })
            .await?;

        Ok(deleted_count.try_into().unwrap_or(0))
    }

    async fn note_exists(&self, note_id: NoteId) -> Result<bool, DatabaseError> {
        let count: i64 = self
            .execute_query("check note existence", move |conn| {
                use schema::notes::dsl::{id, notes};
                let count =
                    notes.filter(id.eq(&note_id.as_bytes()[..])).count().get_result(conn)?;
                Ok(count)
            })
            .await?;

        Ok(count > 0)
    }
}
