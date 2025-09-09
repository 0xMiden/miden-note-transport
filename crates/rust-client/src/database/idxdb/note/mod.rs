use alloc::{string::ToString, vec::Vec};

use chrono::{DateTime, Utc};
use miden_objects::note::{NoteHeader, NoteId, NoteTag};
use serde_wasm_bindgen::from_value;
use wasm_bindgen_futures::JsFuture;

use crate::database::{DatabaseError, StoredNote};

mod js_bindings;
use js_bindings::{
    idxdb_get_fetched_notes_for_tag, idxdb_get_stored_note, idxdb_get_stored_notes_for_tag,
    idxdb_note_fetched, idxdb_record_fetched_note, idxdb_store_note,
};

mod models;
use models::{FetchedNoteIdxdbObject, StoredNoteIdxdbObject};

pub(crate) mod utils;
use utils::{
    deserialize_note_header, deserialize_note_id, serialize_note_header, serialize_note_id,
};

// Note operations
pub async fn store_note(
    header: &NoteHeader,
    encrypted_data: &[u8],
    created_at: DateTime<Utc>,
) -> Result<(), DatabaseError> {
    let header_bytes = serialize_note_header(header)?;
    let note_id_bytes = serialize_note_id(&header.id())?;
    let created_at_str = created_at.to_rfc3339();

    let js_value = JsFuture::from(idxdb_store_note(
        note_id_bytes,
        header_bytes,
        encrypted_data.to_vec(),
        created_at_str,
    ))
    .await
    .map_err(|e| DatabaseError::Protocol(format!("Failed to store note: {:?}", e)))?;

    if js_value.is_undefined() {
        Ok(())
    } else {
        Err(DatabaseError::Protocol("Failed to store note".to_string()))
    }
}

pub async fn get_stored_note(note_id: &NoteId) -> Result<Option<StoredNote>, DatabaseError> {
    let note_id_bytes = serialize_note_id(note_id)?;

    let js_value = JsFuture::from(idxdb_get_stored_note(note_id_bytes))
        .await
        .map_err(|e| DatabaseError::Protocol(format!("Failed to get stored note: {:?}", e)))?;

    if js_value.is_undefined() {
        return Ok(None);
    }

    let note_data: StoredNoteIdxdbObject = from_value(js_value).map_err(|e| {
        DatabaseError::Encoding(format!("Failed to deserialize note data: {:?}", e))
    })?;

    let header = deserialize_note_header(&note_data.header)?;
    let created_at = DateTime::parse_from_rfc3339(&note_data.created_at)
        .map_err(|e| DatabaseError::Encoding(format!("Invalid timestamp: {}", e)))?
        .with_timezone(&Utc);

    Ok(Some(StoredNote {
        header,
        details: note_data.details,
        created_at,
    }))
}

pub async fn get_stored_notes_for_tag(tag: NoteTag) -> Result<Vec<StoredNote>, DatabaseError> {
    let tag_u32 = tag.as_u32();

    let js_value = JsFuture::from(idxdb_get_stored_notes_for_tag(tag_u32)).await.map_err(|e| {
        DatabaseError::Protocol(format!("Failed to get stored notes for tag: {:?}", e))
    })?;

    let notes_data: Vec<StoredNoteIdxdbObject> = from_value(js_value).map_err(|e| {
        DatabaseError::Encoding(format!("Failed to deserialize notes data: {:?}", e))
    })?;

    let mut notes = Vec::new();
    for note_data in notes_data {
        let header = deserialize_note_header(&note_data.header)?;
        let created_at = DateTime::parse_from_rfc3339(&note_data.created_at)
            .map_err(|e| DatabaseError::Encoding(format!("Invalid timestamp: {}", e)))?
            .with_timezone(&Utc);

        notes.push(StoredNote {
            header,
            details: note_data.details,
            created_at,
        });
    }

    Ok(notes)
}

pub async fn record_fetched_note(note_id: &NoteId, tag: NoteTag) -> Result<(), DatabaseError> {
    let note_id_bytes = serialize_note_id(note_id)?;
    let tag_u32 = tag.as_u32();
    let fetched_at = Utc::now().to_rfc3339();

    let js_value = JsFuture::from(idxdb_record_fetched_note(note_id_bytes, tag_u32, fetched_at))
        .await
        .map_err(|e| DatabaseError::Protocol(format!("Failed to record fetched note: {:?}", e)))?;

    if js_value.is_undefined() {
        Ok(())
    } else {
        Err(DatabaseError::Protocol("Failed to record fetched note".to_string()))
    }
}

pub async fn note_fetched(note_id: &NoteId) -> Result<bool, DatabaseError> {
    let note_id_bytes = serialize_note_id(note_id)?;

    let js_value = JsFuture::from(idxdb_note_fetched(note_id_bytes)).await.map_err(|e| {
        DatabaseError::Protocol(format!("Failed to check if note fetched: {:?}", e))
    })?;

    if js_value.is_undefined() {
        Ok(false)
    } else {
        let result: bool = from_value(js_value).map_err(|e| {
            DatabaseError::Encoding(format!("Failed to deserialize boolean: {:?}", e))
        })?;
        Ok(result)
    }
}

pub async fn get_fetched_notes_for_tag(tag: NoteTag) -> Result<Vec<NoteId>, DatabaseError> {
    let tag_u32 = tag.as_u32();

    let js_value = JsFuture::from(idxdb_get_fetched_notes_for_tag(tag_u32)).await.map_err(|e| {
        DatabaseError::Protocol(format!("Failed to get fetched notes for tag: {:?}", e))
    })?;

    let fetched_data: Vec<FetchedNoteIdxdbObject> = from_value(js_value).map_err(|e| {
        DatabaseError::Encoding(format!("Failed to deserialize fetched notes data: {:?}", e))
    })?;

    let mut note_ids = Vec::new();
    for data in fetched_data {
        let note_id = deserialize_note_id(&data.note_id)?;
        note_ids.push(note_id);
    }

    Ok(note_ids)
}
