use chrono::{DateTime, Utc};
use miden_objects::utils::Serializable;
pub use miden_objects::{
    Felt,
    account::AccountId,
    block::BlockNumber,
    note::{Note, NoteDetails, NoteHeader, NoteId, NoteInclusionProof, NoteTag, NoteType},
};
use serde::{Deserialize, Serialize};

/// A note stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredNote {
    /// Note header
    #[serde(
        serialize_with = "serialize_note_header",
        deserialize_with = "deserialize_note_header"
    )]
    pub header: NoteHeader,
    /// Note details
    ///
    /// Can be encrypted.
    pub details: Vec<u8>,
    /// Reference timestamp
    pub created_at: DateTime<Utc>,
}

impl TryFrom<StoredNote> for miden_private_transport_proto::TransportNoteTimestamped {
    type Error = anyhow::Error;

    fn try_from(snote: StoredNote) -> Result<Self, Self::Error> {
        let nanos = snote.created_at.timestamp_subsec_nanos();
        let nanos_i32 = nanos
            .try_into()
            .map_err(|e| anyhow::anyhow!("Timestamp nanoseconds too large: {e}"))?;

        let pnote = miden_private_transport_proto::TransportNote {
            header: snote.header.to_bytes(),
            details: snote.details,
        };

        let ptimestamp = prost_types::Timestamp {
            seconds: snote.created_at.timestamp(),
            nanos: nanos_i32,
        };

        Ok(Self {
            note: Some(pnote),
            timestamp: Some(ptimestamp),
        })
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
