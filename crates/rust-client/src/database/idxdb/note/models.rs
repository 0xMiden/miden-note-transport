use alloc::{string::String, vec::Vec};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredNoteIdxdbObject {
    pub header: Vec<u8>,
    pub details: Vec<u8>,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchedNoteIdxdbObject {
    pub note_id: Vec<u8>,
    pub tag: u32,
    pub fetched_at: String,
}
