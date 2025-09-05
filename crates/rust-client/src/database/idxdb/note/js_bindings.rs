use alloc::{string::String, vec::Vec};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{js_sys, wasm_bindgen};

// Note IndexedDB Operations
#[wasm_bindgen(module = "/src/database/idxdb/js/notes.js")]
extern "C" {
    // STORED NOTES
    // ================================================================================================

    #[wasm_bindgen(js_name = storeNote)]
    pub fn idxdb_store_note(
        note_id: Vec<u8>,
        header: Vec<u8>,
        details: Vec<u8>,
        created_at: String,
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getStoredNote)]
    pub fn idxdb_get_stored_note(note_id: Vec<u8>) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getStoredNotesForTag)]
    pub fn idxdb_get_stored_notes_for_tag(tag: u32) -> js_sys::Promise;

    // FETCHED NOTES
    // ================================================================================================

    #[wasm_bindgen(js_name = recordFetchedNote)]
    pub fn idxdb_record_fetched_note(
        note_id: Vec<u8>,
        tag: u32,
        fetched_at: String,
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = noteFetched)]
    pub fn idxdb_note_fetched(note_id: Vec<u8>) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getFetchedNotesForTag)]
    pub fn idxdb_get_fetched_notes_for_tag(tag: u32) -> js_sys::Promise;
}
