//!# Module Overview
//!
//! This module provides core data structures for the Miden Private Transport Web Client that are
//! exposed to JavaScript via `wasm_bindgen`. These structs serve as wrappers around native objects
//! from the Miden repositories, enabling interaction with the transport layer from a web browser.
//!
//! ## Purpose
//!
//! This module is designed to enable developers to work with core transport layer objects directly
//! from JavaScript in a browser environment. By exposing Rust-native functionality via
//! `wasm_bindgen`, it ensures that the web-based use of the transport client maintains the same
//! functionality as the Rust-native experience.
//!
//! ## Usage
//!
//! The modules provide Rust structs and methods that are exposed to JavaScript via `wasm_bindgen`.
//! These bindings allow developers to create and manipulate transport layer objects in JavaScript,
//! including addresses, notes, note IDs, and note tags.

#![allow(clippy::return_self_not_must_use)]

pub mod account_id;
pub mod address;
pub mod felt;
pub mod fungible_asset;
pub mod note;
pub mod note_assets;
pub mod note_execution_hint;
pub mod note_execution_mode;
pub mod note_id;
pub mod note_inputs;
pub mod note_metadata;
pub mod note_recipient;
pub mod note_script;
pub mod note_tag;
pub mod note_type;
pub mod word;
