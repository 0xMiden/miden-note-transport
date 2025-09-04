use chrono::{DateTime, Utc};
use miden_objects::utils::Serializable;
pub use miden_objects::{
    Felt,
    account::AccountId,
    block::BlockNumber,
    note::{Note, NoteDetails, NoteHeader, NoteId, NoteInclusionProof, NoteTag, NoteType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NoteStatus {
    Sent,
    Duplicate,
}

/// A note stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredNote {
    #[serde(
        serialize_with = "serialize_note_header",
        deserialize_with = "deserialize_note_header"
    )]
    pub header: NoteHeader,
    pub details: Vec<u8>,
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

pub fn random_note_id() -> NoteId {
    use miden_objects::{Felt, Word};
    use rand::Rng;

    let mut rng = rand::rng();

    let recipient = Word::from([
        Felt::new(rng.random::<u64>()),
        Felt::new(rng.random::<u64>()),
        Felt::new(rng.random::<u64>()),
        Felt::new(rng.random::<u64>()),
    ]);
    let asset_commitment = Word::from([
        Felt::new(rng.random::<u64>()),
        Felt::new(rng.random::<u64>()),
        Felt::new(rng.random::<u64>()),
        Felt::new(rng.random::<u64>()),
    ]);

    NoteId::new(recipient, asset_commitment)
}

pub const TEST_TAG: u32 = 3_221_225_472;
pub fn test_note_header() -> NoteHeader {
    use miden_objects::{
        Felt,
        account::AccountId,
        note::{NoteExecutionHint, NoteMetadata, NoteType},
        testing::account_id::ACCOUNT_ID_MAX_ZEROES,
    };

    let id = random_note_id();
    let sender = AccountId::try_from(ACCOUNT_ID_MAX_ZEROES).unwrap();
    let note_type = NoteType::Private;
    let tag = NoteTag::from_account_id(sender);
    let aux = Felt::try_from(0xffff_ffff_0000_0000u64).unwrap();
    let execution_hint = NoteExecutionHint::None;

    let metadata = NoteMetadata::new(sender, note_type, tag, execution_hint, aux).unwrap();

    NoteHeader::new(id, metadata)
}
