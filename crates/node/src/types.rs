use chrono::{DateTime, Utc};
use miden_objects::utils::Serializable;
pub use miden_objects::{
    Felt,
    account::AccountId,
    block::BlockNumber,
    note::{Note, NoteDetails, NoteHeader, NoteId, NoteInclusionProof, NoteTag, NoteType},
};

/// A note stored in the database
#[derive(Debug, Clone)]
pub struct StoredNote {
    /// Note header
    pub header: NoteHeader,
    /// Note details
    ///
    /// Can be encrypted.
    pub details: Vec<u8>,
    /// Reference timestamp
    pub created_at: DateTime<Utc>,
}

impl From<StoredNote> for miden_private_transport_proto::TransportNote {
    fn from(snote: StoredNote) -> Self {
        Self {
            header: snote.header.to_bytes(),
            details: snote.details,
        }
    }
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
