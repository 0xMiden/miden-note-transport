use chrono::{DateTime, Utc};
use miden_lib::{account::wallets::BasicWallet, note::create_p2id_note};
use miden_objects::{
    account::{AccountBuilder, AccountStorageMode},
    crypto::rand::RpoRandomCoin,
};
use miden_testing::Auth;
use serde::{Deserialize, Serialize};

// Use miden-objects
pub use miden_objects::note::{Note, NoteDetails, NoteHeader, NoteId, NoteTag, NoteType};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedDetails(pub Vec<u8>);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NoteStatus {
    Sent,
    Marked,
    Duplicate,
}

impl From<Vec<u8>> for EncryptedDetails {
    fn from(value: Vec<u8>) -> Self {
        EncryptedDetails(value)
    }
}

impl UserId {
    pub fn new(id: String) -> Self {
        UserId(id)
    }

    /// Creates a random UUID v4
    pub fn random() -> Self {
        Self::new(uuid::Uuid::new_v4().to_string())
    }
}

impl From<String> for UserId {
    fn from(value: String) -> Self {
        UserId(value)
    }
}

impl From<miden_transport_proto::UserId> for UserId {
    fn from(proto: miden_transport_proto::UserId) -> Self {
        UserId(proto.value)
    }
}

impl From<UserId> for miden_transport_proto::UserId {
    fn from(id: UserId) -> Self {
        miden_transport_proto::UserId { value: id.0 }
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for EncryptedDetails {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for EncryptedDetails {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Note stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredNote {
    #[serde(
        serialize_with = "serialize_note_header",
        deserialize_with = "deserialize_note_header"
    )]
    pub header: NoteHeader,
    pub encrypted_data: EncryptedDetails,
    pub created_at: DateTime<Utc>,
    pub received_by: Option<Vec<String>>,
}

/// Information about a note in API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteInfo {
    #[serde(
        serialize_with = "serialize_note_header",
        deserialize_with = "deserialize_note_header"
    )]
    pub header: NoteHeader,
    pub encrypted_data: EncryptedDetails,
    pub created_at: DateTime<Utc>,
}

/// Server health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
}

/// Server statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub total_notes: u64,
    pub total_tags: u64,
    pub notes_per_tag: Vec<TagStats>,
}

/// Statistics for a specific tag
#[derive(Debug, Serialize, Deserialize)]
pub struct TagStats {
    #[serde(
        serialize_with = "serialize_note_tag",
        deserialize_with = "deserialize_note_tag"
    )]
    pub tag: NoteTag,
    pub note_count: u64,
    pub last_activity: Option<DateTime<Utc>>,
}

fn serialize_note_tag<S>(tag: &NoteTag, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format!("{:08x}", tag.as_u32()))
}

fn deserialize_note_tag<'de, D>(deserializer: D) -> Result<NoteTag, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let s = String::deserialize(deserializer)?;
    u32::from_str_radix(&s, 16)
        .map(|uint| uint.into())
        .map_err(|e| D::Error::custom(format!("Failed to parse NoteTag for u32: {e:?}")))
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
        D::Error::custom(format!(
            "Failed to deserialize NoteHeader from bytes: {e:?}"
        ))
    })
}

pub fn random_note_id() -> NoteId {
    use miden_objects::{Digest, Felt, Word};
    use rand::Rng;

    let mut rng = rand::rng();

    let recipient_word = Word::from([
        Felt::new(rng.random::<u64>()),
        Felt::new(rng.random::<u64>()),
        Felt::new(rng.random::<u64>()),
        Felt::new(rng.random::<u64>()),
    ]);
    let asset_commitment_word = Word::from([
        Felt::new(rng.random::<u64>()),
        Felt::new(rng.random::<u64>()),
        Felt::new(rng.random::<u64>()),
        Felt::new(rng.random::<u64>()),
    ]);

    let recipient = Digest::from(recipient_word);
    let asset_commitment = Digest::from(asset_commitment_word);

    NoteId::new(recipient, asset_commitment)
}

pub const TEST_TAG: u32 = 3221225472;
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

pub fn mock_note_p2id() -> miden_objects::note::Note {
    use rand::Rng;
    let mut rng = rand::rng();
    let (account, _seed) = AccountBuilder::new(rng.random())
        .storage_mode(AccountStorageMode::Public)
        .with_component(BasicWallet)
        .with_auth_component(Auth::BasicAuth)
        .build()
        .unwrap();
    let mut rng = RpoRandomCoin::new(Default::default());
    create_p2id_note(
        account.id(),
        account.id(),
        vec![],
        NoteType::Private,
        Default::default(),
        &mut rng,
    )
    .unwrap()
}
