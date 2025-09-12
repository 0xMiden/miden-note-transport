use alloc::{string::ToString, vec::Vec};

use chrono::{DateTime, Utc};
use miden_objects::{
    note::{NoteHeader, NoteId, NoteTag},
    utils::{Deserializable, Serializable},
};
use rusqlite::{Connection, params};

use super::{DatabaseError, StoredNote};
use crate::{insert_sql, subst};

/// Note-related database operations
pub struct NoteOperations;

impl NoteOperations {
    /// Store a note in the database
    pub fn store_note(
        conn: &mut Connection,
        header: &NoteHeader,
        details: &[u8],
        created_at: DateTime<Utc>,
    ) -> Result<(), DatabaseError> {
        const STORE_NOTE_QUERY: &str = insert_sql!(
            stored_notes {
                note_id,
                tag,
                header,
                details,
                created_at
            } | REPLACE
        );

        let note_id = header.id();
        let tag = header.metadata().tag();
        let header_bytes = header.to_bytes();
        let details = details.to_vec();

        conn.execute(
            STORE_NOTE_QUERY,
            params![
                &note_id.as_bytes()[..],
                i64::from(tag.as_u32()),
                &header_bytes,
                &details,
                created_at.timestamp_micros()
            ],
        )?;
        Ok(())
    }

    /// Get a stored note by ID
    pub fn get_stored_note(
        conn: &mut Connection,
        note_id: &NoteId,
    ) -> Result<Option<StoredNote>, DatabaseError> {
        let note_id_bytes = note_id.as_bytes().to_vec();

        let mut stmt = conn.prepare(
            "SELECT tag, header, details, created_at FROM stored_notes WHERE note_id = ?",
        )?;
        let mut rows = stmt.query(params![&note_id_bytes])?;

        if let Some(row) = rows.next()? {
            let header_bytes: Vec<u8> = row.get("header")?;
            let details: Vec<u8> = row.get("details")?;
            let created_at_micros: i64 = row.get("created_at")?;

            let header = NoteHeader::read_from_bytes(&header_bytes)
                .map_err(|e| DatabaseError::Encoding(e.to_string()))?;
            let created_at =
                DateTime::from_timestamp_micros(created_at_micros).ok_or_else(|| {
                    DatabaseError::Encoding(format!(
                        "Invalid timestamp microseconds: {created_at_micros}"
                    ))
                })?;

            Ok(Some(StoredNote { header, details, created_at }))
        } else {
            Ok(None)
        }
    }

    /// Get all stored notes for a specific tag
    pub fn get_stored_notes_for_tag(
        conn: &mut Connection,
        tag: NoteTag,
    ) -> Result<Vec<StoredNote>, DatabaseError> {
        let mut stmt = conn.prepare("SELECT note_id, header, details, created_at FROM stored_notes WHERE tag = ? ORDER BY created_at ASC")?;
        let mut rows = stmt.query(params![i64::from(tag.as_u32())])?;
        let mut notes = Vec::new();

        while let Some(row) = rows.next()? {
            let header_bytes: Vec<u8> = row.get("header")?;
            let details: Vec<u8> = row.get("details")?;
            let created_at_micros: i64 = row.get("created_at")?;

            let header = NoteHeader::read_from_bytes(&header_bytes)
                .map_err(|e| DatabaseError::Encoding(e.to_string()))?;
            let created_at =
                DateTime::from_timestamp_micros(created_at_micros).ok_or_else(|| {
                    DatabaseError::Encoding(format!(
                        "Invalid timestamp microseconds: {created_at_micros}"
                    ))
                })?;

            notes.push(StoredNote { header, details, created_at });
        }

        Ok(notes)
    }

    /// Record that a note has been fetched
    pub fn record_fetched_note(
        conn: &mut Connection,
        note_id: &NoteId,
        tag: NoteTag,
    ) -> Result<(), DatabaseError> {
        const RECORD_FETCHED_QUERY: &str =
            insert_sql!(fetched_notes { note_id, tag, fetched_at } | REPLACE);

        let now = Utc::now();
        let note_id_bytes = note_id.as_bytes().to_vec();

        conn.execute(
            RECORD_FETCHED_QUERY,
            params![&note_id_bytes, i64::from(tag.as_u32()), now.to_rfc3339()],
        )?;
        Ok(())
    }

    /// Check if a note has been fetched
    pub fn note_fetched(conn: &mut Connection, note_id: &NoteId) -> Result<bool, DatabaseError> {
        let note_id_bytes = note_id.as_bytes().to_vec();

        let mut stmt = conn.prepare("SELECT 1 FROM fetched_notes WHERE note_id = ?")?;
        let mut rows = stmt.query(params![&note_id_bytes])?;
        Ok(rows.next()?.is_some())
    }

    /// Get all fetched note IDs for a specific tag
    pub fn get_fetched_notes_for_tag(
        conn: &mut Connection,
        tag: NoteTag,
    ) -> Result<Vec<NoteId>, DatabaseError> {
        let mut stmt = conn
            .prepare("SELECT note_id FROM fetched_notes WHERE tag = ? ORDER BY fetched_at ASC")?;
        let mut rows = stmt.query(params![i64::from(tag.as_u32())])?;
        let mut note_ids = Vec::new();

        while let Some(row) = rows.next()? {
            let note_id_bytes: Vec<u8> = row.get("note_id")?;
            let note_id = NoteId::read_from_bytes(&note_id_bytes)
                .map_err(|e| DatabaseError::Encoding(e.to_string()))?;
            note_ids.push(note_id);
        }

        Ok(note_ids)
    }
}
