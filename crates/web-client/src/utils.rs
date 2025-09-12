use miden_client::utils::{Deserializable, Serializable};
use miden_objects::utils::SliceReader;
use miden_private_transport_client::test_utils::{
    mock_address as rc_mock_address,
    mock_note_p2id_with_addresses as rc_mock_note_p2id_with_addresses,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::Uint8Array;

use crate::{
    js_error_with_context,
    models::{address::Address, note::Note, note_tag::NoteTag},
};

/// Serializes any value that implements `Serializable` into a `Uint8Array`.
pub fn serialize_to_uint8array<T: Serializable>(value: &T) -> Uint8Array {
    let mut buffer = Vec::new();
    value.write_into(&mut buffer);
    Uint8Array::from(&buffer[..])
}

/// Deserializes a `Uint8Array` into any type that implements `Deserializable`.
pub fn deserialize_from_uint8array<T: Deserializable>(bytes: &Uint8Array) -> Result<T, JsValue> {
    let vec = bytes.to_vec();
    let mut reader = SliceReader::new(&vec);
    T::read_from(&mut reader).map_err(|e| js_error_with_context(e, "failed to deserialize"))
}

/// Parse a bech32 address string and create an Address object
#[wasm_bindgen]
pub fn parse_bech32_address(bech32_str: &str) -> Result<Address, JsValue> {
    let (_, address) = miden_objects::address::Address::from_bech32(bech32_str)
        .map_err(|e| JsValue::from_str(&format!("Invalid bech32 address {}: {}", bech32_str, e)))?;

    // Convert the native Address to our WASM-compatible Address
    Ok(address.into())
}

/// Create a NoteTag from an integer
#[wasm_bindgen]
pub fn create_note_tag_from_int(tag_value: u32) -> NoteTag {
    let native_tag = miden_private_transport_client::types::NoteTag::from(tag_value);
    native_tag.into()
}

/// Parse a hex string and create a Note object
#[wasm_bindgen]
pub fn parse_hex_note(hex_str: &str) -> Result<Note, JsValue> {
    let bytes =
        hex::decode(hex_str).map_err(|e| JsValue::from_str(&format!("Invalid hex data: {}", e)))?;

    let note = miden_objects::note::Note::read_from_bytes(&bytes)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize Note: {}", e)))?;

    // Convert the native Note to our WASM-compatible Note
    Ok(note.into())
}

/// Get note information as a JSON string for display
#[wasm_bindgen]
pub fn get_note_info(note: &Note) -> Result<String, JsValue> {
    // Convert WASM Note back to native Note for inspection
    let native_note: miden_objects::note::Note = note.into();

    let note_info = serde_json::json!({
        "header": {
            "version": 1,
            "tag": native_note.header().metadata().tag().as_u32(),
            "sender": format!("{}", native_note.header().metadata().sender()),
        },
        "script_type": "P2ID",
        "script_has_externals": true,
        "inputs_count": native_note.inputs().values().len(),
        "note_id": format!("{}", native_note.id()),
    });

    serde_json::to_string_pretty(&note_info)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize note info: {}", e)))
}

/// Get note tag as integer
#[wasm_bindgen]
pub fn get_note_tag(note: &Note) -> u32 {
    let native_note: miden_objects::note::Note = note.into();
    native_note.header().metadata().tag().as_u32()
}

/// Get note sender address as bech32 string
#[wasm_bindgen]
pub fn get_note_sender_bech32(note: &Note) -> Result<String, JsValue> {
    let native_note: miden_objects::note::Note = note.into();
    let sender = native_note.header().metadata().sender();
    Ok(format!("{}", sender))
}

// TEST UTILS
// ================================================================================================

#[wasm_bindgen(js_name = "mockAddress")]
pub fn mock_address() -> Result<Address, JsValue> {
    let native_address = rc_mock_address();
    Ok(native_address.into())
}

#[wasm_bindgen(js_name = "mockNoteP2IDWithAddresses")]
pub fn mock_note_p2id_with_addresses(sender: &Address, target: &Address) -> Result<Note, JsValue> {
    // Convert web-client Address to rust-client Address
    let rc_sender: miden_objects::address::Address = sender.into();
    let rc_target: miden_objects::address::Address = target.into();

    let native_note = rc_mock_note_p2id_with_addresses(&rc_sender, &rc_target);
    Ok(native_note.into())
}
