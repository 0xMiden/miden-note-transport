use miden_objects::account::AccountId;
use miden_objects::note::{NoteExecutionHint, NoteHeader, NoteId, NoteMetadata, NoteTag, NoteType};
use miden_objects::testing::account_id::ACCOUNT_ID_MAX_ZEROES;
use miden_objects::{Felt, Word};
use rand::Rng;

/// Generate a random [`NoteId`]
pub fn random_note_id() -> NoteId {
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

/// Generate a private [`NoteHeader`] with random sender
pub fn test_note_header() -> NoteHeader {
    let id = random_note_id();
    let sender = AccountId::try_from(ACCOUNT_ID_MAX_ZEROES).unwrap();
    let note_type = NoteType::Private;
    let tag = NoteTag::from_account_id(sender);
    let aux = Felt::try_from(0xffff_ffff_0000_0000u64).unwrap();
    let execution_hint = NoteExecutionHint::None;

    let metadata = NoteMetadata::new(sender, note_type, tag, execution_hint, aux).unwrap();

    NoteHeader::new(id, metadata)
}
