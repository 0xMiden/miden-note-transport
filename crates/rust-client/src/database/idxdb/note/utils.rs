use alloc::vec::Vec;

use miden_objects::{
    note::{NoteHeader, NoteId},
    utils::{Deserializable, Serializable},
};

use crate::database::DatabaseError;

pub fn serialize_note_header(header: &NoteHeader) -> Result<Vec<u8>, DatabaseError> {
    Ok(header.to_bytes())
}

pub fn deserialize_note_header(data: &[u8]) -> Result<NoteHeader, DatabaseError> {
    NoteHeader::read_from_bytes(data)
        .map_err(|e| DatabaseError::Encoding(format!("Failed to deserialize NoteHeader: {}", e)))
}

pub fn serialize_note_id(note_id: &NoteId) -> Result<Vec<u8>, DatabaseError> {
    Ok(note_id.to_bytes())
}

pub fn deserialize_note_id(data: &[u8]) -> Result<NoteId, DatabaseError> {
    NoteId::read_from_bytes(data)
        .map_err(|e| DatabaseError::Encoding(format!("Failed to deserialize NoteId: {}", e)))
}
