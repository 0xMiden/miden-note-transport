use chrono::Utc;
use rusqlite::Connection;

use super::{DatabaseError, DatabaseStats};

/// Statistics-related database operations
pub struct StatsOperations;

impl StatsOperations {
    /// Get basic database statistics
    pub fn get_stats(conn: &mut Connection) -> Result<DatabaseStats, DatabaseError> {
        let fetched_notes_count: u64 =
            conn.query_row("SELECT COUNT(*) FROM fetched_notes", [], |row| row.get(0))?;

        let stored_notes_count: u64 =
            conn.query_row("SELECT COUNT(*) FROM stored_notes", [], |row| row.get(0))?;

        let unique_tags_count: u64 =
            conn.query_row("SELECT COUNT(DISTINCT tag) FROM stored_notes", [], |row| row.get(0))?;

        Ok(DatabaseStats {
            fetched_notes_count,
            stored_notes_count,
            unique_tags_count,
        })
    }

    /// Clean up old data based on retention period
    pub fn cleanup_old_data(
        conn: &mut Connection,
        retention_days: u32,
    ) -> Result<u64, DatabaseError> {
        let cutoff_date = Utc::now() - chrono::Duration::days(i64::from(retention_days));

        // Clean up old fetched notes
        let fetched_changes = conn.execute(
            "DELETE FROM fetched_notes WHERE fetched_at < ?",
            rusqlite::params![cutoff_date.to_rfc3339()],
        )?;

        // Clean up old stored notes
        let stored_changes = conn.execute(
            "DELETE FROM stored_notes WHERE created_at < ?",
            rusqlite::params![cutoff_date.to_rfc3339()],
        )?;

        Ok((fetched_changes + stored_changes) as u64)
    }
}
