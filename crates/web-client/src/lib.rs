extern crate alloc;

pub mod models;
pub mod utils;

use miden_private_transport_client::{
    GrpcClient, TransportLayerClient,
    database::{Database, idxdb::IndexedDb},
    types::NoteTag as NativeNoteTag,
};
use wasm_bindgen::prelude::*;

use crate::models::{address::Address, note::Note, note_id::NoteId, note_tag::NoteTag};

#[wasm_bindgen]
pub struct TransportLayerWebClient {
    inner: Option<TransportLayerClient>,
}

#[wasm_bindgen]
impl TransportLayerWebClient {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        TransportLayerWebClient { inner: None }
    }

    #[wasm_bindgen]
    pub async fn connect(&mut self, url: &str) -> Result<(), JsValue> {
        let transport_client = GrpcClient::connect(url.to_string(), 1000)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to connect to transport: {:?}", e)))?;

        let indexed_db = IndexedDb::init()
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to initialize IndexedDB: {e:?}")))?;
        let db = Database::new(Box::new(indexed_db));

        let client = TransportLayerClient::new(Box::new(transport_client), db, vec![]);

        self.inner = Some(client);
        Ok(())
    }

    /// Send a note to the transport layer
    #[wasm_bindgen(js_name = "sendNote")]
    pub async fn send_note(&mut self, note: &Note, address: &Address) -> Result<NoteId, JsValue> {
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| JsValue::from_str("Client not initialized. Call init() first."))?;

        let native_note: miden_objects::note::Note = note.into();
        let native_address: miden_objects::address::Address = address.into();
        let note_id = native_note.id();

        inner
            .send_note(native_note, &native_address)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to send note: {:?}", e)))?;

        // Return the note ID from the note that was sent
        Ok(note_id.into())
    }

    /// Fetch notes from the transport layer for one or more tags
    #[wasm_bindgen(js_name = "fetchNotes")]
    pub async fn fetch_notes(&mut self, tags: Vec<NoteTag>) -> Result<Vec<Note>, JsValue> {
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| JsValue::from_str("Client not initialized. Call connect() first."))?;

        let native_tags: Vec<NativeNoteTag> = tags.iter().map(|tag| tag.into()).collect();

        let notes = inner
            .fetch_notes(&native_tags)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch notes: {e:?}")))?;

        // Convert native notes to JS note objects
        let js_notes: Vec<Note> = notes.into_iter().map(|native_note| native_note.into()).collect();

        Ok(js_notes)
    }
}

// ERROR HANDLING HELPERS
// ================================================================================================

fn js_error_with_context<T>(err: T, context: &str) -> JsValue
where
    T: core::error::Error,
{
    let mut error_string = context.to_string();
    let mut source = Some(&err as &dyn core::error::Error);
    while let Some(err) = source {
        error_string.push_str(&format!(": {err}"));
        source = err.source();
    }
    JsValue::from(error_string)
}
