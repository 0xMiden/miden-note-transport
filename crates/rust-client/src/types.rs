use alloc::{string::ToString, vec::Vec};

use chrono::{DateTime, Utc};
use miden_objects::address::Address;
pub use miden_objects::{
    Felt,
    account::AccountId,
    block::BlockNumber,
    note::{
        Note, NoteDetails, NoteHeader, NoteId, NoteInclusionProof, NoteMetadata, NoteTag, NoteType,
    },
};
use serde::{Deserialize, Serialize};

/// A note stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredNote {
    #[serde(
        serialize_with = "serialize_note_header",
        deserialize_with = "deserialize_note_header"
    )]
    /// Note header
    pub header: NoteHeader,
    /// Note details, can be encrypted
    pub details: Vec<u8>,
    /// Note reference cursor
    pub cursor: u64,
    /// Note fetched-at timestamp
    pub received_at: DateTime<Utc>,
}

/// Information about a note in API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteInfo {
    #[serde(
        serialize_with = "serialize_note_header",
        deserialize_with = "deserialize_note_header"
    )]
    /// Note header
    pub header: NoteHeader,
    /// Note details, can be encrypted
    pub details: Vec<u8>,
    /// Note reference cursor
    pub cursor: u64,
}

/// Helper converter from [`prost_types::Timestamp`] to `DateTime<Utc>`
pub fn proto_timestamp_to_datetime(pts: prost_types::Timestamp) -> anyhow::Result<DateTime<Utc>> {
    let dts = DateTime::from_timestamp(
        pts.seconds,
        pts.nanos
            .try_into()
            .map_err(|_| anyhow::anyhow!("Negative timestamp nanoseconds".to_string()))?,
    )
    .ok_or_else(|| anyhow::anyhow!("Invalid timestamp".to_string()))?;

    Ok(dts)
}

fn serialize_note_header<S>(note_header: &NoteHeader, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use miden_objects::utils::Serializable;
    serializer.serialize_bytes(&note_header.to_bytes())
}

fn deserialize_note_header<'de, D>(deserializer: D) -> Result<NoteHeader, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use miden_objects::utils::Deserializable;
    use serde::de::Error;
    let bytes = Vec::<u8>::deserialize(deserializer)?;
    NoteHeader::read_from_bytes(&bytes).map_err(|e| {
        D::Error::custom(format!("Failed to deserialize NoteHeader from bytes: {e:?}"))
    })
}

/// Get underlying account ID of an `Address::AccountId`
pub fn address_to_account_id(address: &Address) -> Option<AccountId> {
    if let Address::AccountId(aia) = address {
        Some(aia.id())
    } else {
        None
    }
}
