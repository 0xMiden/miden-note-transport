use miden_objects::{note::{NoteDetails, NoteHeader}, utils::Serializable};
use miden_private_transport_client::types::{mock_note_p2id, test_note_header};
use rand::Rng;

const DETAILS_LEN_AVG: usize = 1500;
const DETAILS_LEN_DEV: usize = 100;
pub const TAG_LOCAL_ANY: u32 = 0xc000_0000;

pub fn generate_dummy_notes(n: usize) -> Vec<(NoteHeader, Vec<u8>)> {
    let mut rng = rand::rng();
    let mut tag = TAG_LOCAL_ANY;
    (0..n).map(|_| {
        tag += 1;
        let header = test_note_header(tag.into());
        let details = vec![0u8; DETAILS_LEN_AVG + rng.random_range(0..(DETAILS_LEN_DEV * 2 - DETAILS_LEN_DEV))];
        (header, details)
    }).collect()
}
